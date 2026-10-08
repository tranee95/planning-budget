//! Вход, создание, смена пароля и сброс хранилища.
//! Функции синхронные и тяжёлые (Argon2id): команды вызывают их через `state::blocking`.
//!
//! Каждая функция на всё время работы держит `state.vault()`: операции над `vault.json`
//! идут строго по очереди, параллельные команды не видят файл в середине записи.

use std::fs;
use std::io::ErrorKind;

use chrono::{DateTime, Utc};
use planning_budget_storage::Db;
use planning_budget_vault::{KdfParams, VaultError, VaultStore};
use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::AppError;
use crate::dto::VaultStatusDto;
use crate::state::AppState;

mod recovery_file;
mod rekey;

pub use recovery_file::save_recovery_code;
pub use rekey::rekey;
use rekey::{complete_unlock, open_db_finishing_rekey};

/// Фраза подтверждения сброса; сравнение без учёта регистра и крайних пробелов.
pub const RESET_PHRASE: &str = "удалить все данные";

const AUTOLOCK_KEY: &str = "security.autolock_minutes";
const LOCK_ON_MINIMIZE_KEY: &str = "security.lock_on_minimize";
const DEFAULT_AUTOLOCK_MINUTES: u32 = 5;

pub fn status(state: &AppState, now: DateTime<Utc>) -> Result<VaultStatusDto, AppError> {
    let store = state.vault();
    let status = store.status(now).map_err(|e| vault_err(&store, now, e))?;
    Ok(VaultStatusDto {
        exists: status.exists,
        locked: !state.is_unlocked(),
        retry_after_ms: secs_to_ms(status.retry_after_secs),
        migration_failed: state.migration_failed(),
    })
}

/// Переносит данные из папки прежнего идентификатора и запоминает итог.
///
/// Ошибка не прерывает запуск: экран входа покажет её и предложит повторить. В лог идёт только
/// вид ошибки ввода-вывода, без путей.
pub fn run_migration(state: &AppState) {
    let Some(old) = state.legacy_dir() else {
        return;
    };
    let result = {
        let _vault = state.vault();
        crate::data_migration::migrate(&old, state.paths().data_dir())
    };
    if let Err(err) = &result {
        tracing::warn!(kind = ?err.kind(), "data migration failed");
    }
    state.set_migration_failed(result.is_err());
}

/// Повторяет перенос данных по кнопке на экране входа.
///
/// # Errors
/// `Io`, если перенос снова не удался (созданное откатывается, старая папка не меняется).
pub fn retry_migration(state: &AppState, now: DateTime<Utc>) -> Result<VaultStatusDto, AppError> {
    run_migration(state);
    if state.migration_failed() {
        return Err(AppError::Io {
            message_key: "errors.io.migration".into(),
        });
    }
    status(state, now)
}

/// Создаёт хранилище и сразу открывает сессию. Возвращает recovery-код для показа.
///
/// `kdf = None` — калибровка под эту машину; тесты передают облегчённые параметры.
pub fn create(
    state: &AppState,
    password: &SecretString,
    now: DateTime<Utc>,
    kdf: Option<KdfParams>,
) -> Result<Zeroizing<String>, AppError> {
    let store = state.vault();
    if state.paths().db_files().iter().any(|p| p.exists())
        && !store.status(now).is_ok_and(|s| s.exists)
    {
        // База без сейфа: ключа к ней нет. Удалять её молча необратимо, решает пользователь.
        return Err(AppError::Conflict {
            message_key: "errors.vault.orphan_database".into(),
        });
    }
    let params = match kdf {
        Some(params) => params,
        None => planning_budget_vault::calibrate().map_err(|e| vault_err(&store, now, e))?,
    };
    let created = store
        .create_with_params(password, params, now)
        .map_err(|e| vault_err(&store, now, e))?;

    let opened = Db::create(&state.paths().db(), created.dek.as_bytes())
        .map_err(AppError::from)
        .and_then(|mut db| {
            db.seed_defaults(now)?;
            Ok(db)
        });
    match opened {
        Ok(db) => {
            finish_unlock(state, db)?;
            state.allow_recovery_save(true);
            // Новый сейф есть: переносить в эту папку больше нечего.
            state.set_migration_failed(false);
            Ok(created.recovery_code.display())
        }
        Err(err) => {
            // Сейф без базы бесполезен и блокирует повторное создание.
            remove_all_files(state, &store);
            Err(err)
        }
    }
}

/// Вход по паролю. Если он довёл до конца прерванный перевыпуск ключа, возвращает новый
/// recovery-код: старый уже не работает, и пользователь должен его сохранить.
pub fn unlock(
    state: &AppState,
    password: &SecretString,
    now: DateTime<Utc>,
) -> Result<Option<Zeroizing<String>>, AppError> {
    let store = state.vault();
    let dek = store
        .unlock(password, now)
        .map_err(|e| vault_err(&store, now, e))?;
    if state.is_unlocked() {
        // Сессия уже открыта: пароль проверен выше, базу заново не открываем.
        return Ok(None);
    }
    let (db, pending) = open_db_finishing_rekey(state, &store, password, &dek, now)?;
    complete_unlock(state, &store, db, pending, now)
}

/// «Забыл пароль»: recovery-код открывает DEK, пароль заменяется на новый.
pub fn unlock_recovery(
    state: &AppState,
    code: &SecretString,
    new_password: &SecretString,
    now: DateTime<Utc>,
) -> Result<Option<Zeroizing<String>>, AppError> {
    let store = state.vault();
    let dek = store
        .reset_password_with_recovery(code, new_password)
        .map_err(|e| vault_err(&store, now, e))?;
    if state.is_unlocked() {
        return Ok(None);
    }
    let (db, pending) = open_db_finishing_rekey(state, &store, new_password, &dek, now)?;
    complete_unlock(state, &store, db, pending, now)
}

pub fn change_password(
    state: &AppState,
    old: &SecretString,
    new: &SecretString,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let store = state.vault();
    store
        .change_password(old, new, now)
        .map_err(|e| vault_err(&store, now, e))
}

/// «Удалить все данные»: пароль и фраза → закрыть сессию → стереть ключи → удалить БД.
///
/// Ключи стираются первыми: после этого данные недоступны, даже если файл базы удалить
/// не получилось. Обратный порядок оставил бы рабочий сейф без базы.
pub fn reset(
    state: &AppState,
    password: &SecretString,
    confirm_phrase: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if confirm_phrase.trim().to_lowercase() != RESET_PHRASE {
        return Err(AppError::Validation {
            message_key: "errors.vault.confirm_phrase".into(),
            field: Some("confirmPhrase".into()),
        });
    }
    let store = state.vault();
    store
        .unlock(password, now)
        .map_err(|e| vault_err(&store, now, e))?;
    state.lock();
    store.destroy().map_err(|e| vault_err(&store, now, e))?;
    for path in state.paths().db_files() {
        remove_if_exists(&path)?;
    }
    // Порога автоблокировки больше нет; тема и масштаб остаются: это не данные бюджета.
    if let Err(err) = crate::prefs::set_autolock_minutes(&state.paths().prefs(), None) {
        tracing::warn!(?err, "autolock mirror not cleared");
    }
    Ok(())
}

/// Читает порог автоблокировки из настроек и начинает сессию.
fn finish_unlock(state: &AppState, db: Db) -> Result<(), AppError> {
    let minutes = db
        .setting(AUTOLOCK_KEY)?
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_AUTOLOCK_MINUTES);
    let lock_on_minimize = db
        .setting(LOCK_ON_MINIMIZE_KEY)?
        .is_some_and(|v| v == "true");
    crate::services::settings::apply_security(
        state,
        u16::try_from(minutes).unwrap_or(u16::MAX),
        lock_on_minimize,
    );
    state.open_session(db);
    Ok(())
}

fn remove_all_files(state: &AppState, store: &VaultStore) {
    for path in state.paths().db_files() {
        let _ = remove_if_exists(&path);
    }
    let _ = store.destroy();
}

fn remove_if_exists(path: &std::path::Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(AppError::Io {
            message_key: "errors.io.delete".into(),
        }),
        _ => Ok(()),
    }
}

fn secs_to_ms(secs: u64) -> u32 {
    u32::try_from(secs.saturating_mul(1000)).unwrap_or(u32::MAX)
}

/// Ошибки сейфа → `AppError`. После неверного пароля берём актуальную задержку из `vault.json`.
fn vault_err(store: &VaultStore, now: DateTime<Utc>, err: VaultError) -> AppError {
    let validation = |key: &str, field: &str| AppError::Validation {
        message_key: key.to_owned(),
        field: Some(field.to_owned()),
    };
    match err {
        VaultError::WrongPassword => AppError::WrongPassword {
            retry_after_ms: store
                .status(now)
                .map_or(0, |s| secs_to_ms(s.retry_after_secs)),
        },
        // Пароль в окне задержки не проверялся: это не «неверный пароль».
        VaultError::Backoff { retry_after_secs } => AppError::TooManyAttempts {
            retry_after_ms: secs_to_ms(retry_after_secs),
        },
        VaultError::WrongRecoveryCode => validation("errors.vault.wrong_recovery_code", "code"),
        VaultError::RecoveryCodeFormat => validation("errors.vault.recovery_code_format", "code"),
        VaultError::PasswordTooShort { .. } => {
            validation("errors.vault.password_too_short", "password")
        }
        VaultError::AlreadyExists => AppError::Conflict {
            message_key: "errors.vault.already_exists".into(),
        },
        VaultError::RekeyPending => AppError::Conflict {
            message_key: "errors.vault.rekey_pending".into(),
        },
        VaultError::NotFound => AppError::NotFound {
            entity: "vault".into(),
            id: 0,
        },
        VaultError::Io(_) => AppError::Io {
            message_key: "errors.io.vault".into(),
        },
        VaultError::Corrupt | VaultError::UnsupportedVersion(_) => {
            tracing::error!(error = %err, "vault file rejected");
            AppError::Io {
                message_key: "errors.vault.corrupt".into(),
            }
        }
        VaultError::Kdf | VaultError::Crypto | VaultError::Random => {
            AppError::internal("vault", &err)
        }
    }
}

#[cfg(test)]
mod tests;

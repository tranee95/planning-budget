//! Вход, создание, смена пароля и сброс хранилища.
//! Функции синхронные и тяжёлые (Argon2id): команды вызывают их через `state::blocking`.
//!
//! Каждая функция на всё время работы держит `state.vault()`: операции над `vault.json`
//! идут строго по очереди, параллельные команды не видят файл в середине записи.

use std::fs;
use std::io::ErrorKind;

use chrono::{DateTime, Utc};
use planning_budget_storage::{Db, StorageError};
use planning_budget_vault::{Dek, KdfParams, VaultError, VaultStore};
use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::AppError;
use crate::dto::VaultStatusDto;
use crate::state::AppState;

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
    })
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
            Ok(created.recovery_code.display())
        }
        Err(err) => {
            // Сейф без базы бесполезен и блокирует повторное создание.
            remove_all_files(state, &store);
            Err(err)
        }
    }
}

pub fn unlock(
    state: &AppState,
    password: &SecretString,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if state.is_unlocked() {
        return Ok(());
    }
    let store = state.vault();
    let dek = store
        .unlock(password, now)
        .map_err(|e| vault_err(&store, now, e))?;
    let db = open_db_finishing_rekey(state, &store, password, &dek, now)?;
    finish_unlock(state, db)
}

/// «Забыл пароль»: recovery-код открывает DEK, пароль заменяется на новый.
pub fn unlock_recovery(
    state: &AppState,
    code: &SecretString,
    new_password: &SecretString,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let store = state.vault();
    let dek = store
        .reset_password_with_recovery(code, new_password)
        .map_err(|e| vault_err(&store, now, e))?;
    if state.is_unlocked() {
        return Ok(());
    }
    let db = open_db_finishing_rekey(state, &store, new_password, &dek, now)?;
    finish_unlock(state, db)
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

/// Пишет recovery-код в файл, выбранный пользователем в нативном диалоге.
/// Разрешено только сразу после выдачи кода (`create`, `rekey`).
pub fn save_recovery_code(
    state: &AppState,
    code: &SecretString,
    pick_path: impl FnOnce() -> Option<std::path::PathBuf>,
) -> Result<bool, AppError> {
    use secrecy::ExposeSecret as _;

    if !state.recovery_save_allowed() {
        return Err(AppError::Conflict {
            message_key: "errors.vault.recovery_save_not_allowed".into(),
        });
    }
    let Some(path) = pick_path() else {
        return Ok(false);
    };
    let text = Zeroizing::new(format!("{}\n", code.expose_secret()));
    fs::write(path, text.as_bytes()).map_err(|_| AppError::Io {
        message_key: "errors.io.recovery_file".into(),
    })?;
    state.allow_recovery_save(false);
    Ok(true)
}

/// Открывает БД при входе. Если прошлый перевыпуск ключа прервался, решает по тому,
/// каким ключом читается база:
/// - старым — `rekey` не дошёл до файла, недоведённые слоты отбрасываются;
/// - новым — база уже перешифрована, слоты доводятся до конца.
///
/// `password` — пароль, под которым сейчас лежит слот `next`. При входе по recovery-коду
/// это не так, и перешифрованную базу открыть нечем: остаточный риск .
fn open_db_finishing_rekey(
    state: &AppState,
    store: &VaultStore,
    password: &SecretString,
    dek: &Dek,
    now: DateTime<Utc>,
) -> Result<Db, AppError> {
    let path = state.paths().db();
    if !store
        .rekey_pending()
        .map_err(|e| vault_err(store, now, e))?
    {
        return Ok(Db::open(&path, dek.as_bytes())?);
    }
    match Db::open(&path, dek.as_bytes()) {
        Ok(db) => {
            store.abort_rekey().map_err(|e| vault_err(store, now, e))?;
            tracing::warn!("interrupted key reissue discarded: database kept its key");
            Ok(db)
        }
        Err(StorageError::KeyRejected) => {
            let interrupted = || AppError::Conflict {
                message_key: "errors.vault.rekey_interrupted".into(),
            };
            let next = store
                .pending_rekey_dek(password)
                .map_err(|_| interrupted())?
                .ok_or_else(interrupted)?;
            let db = Db::open(&path, next.as_bytes())?;
            store.finish_rekey().map_err(|e| vault_err(store, now, e))?;
            tracing::warn!("interrupted key reissue completed; new recovery code was never shown");
            Ok(db)
        }
        Err(e) => Err(e.into()),
    }
}

/// Перевыпуск ключа шифрования: новый DEK, `PRAGMA rekey`, новый recovery-код.
///
/// Порядок защищает от потери данных при сбое: сначала новые слоты пишутся рядом со старыми,
/// потом перешифровывается база, и только после этого старые слоты стираются.
///
/// При любой ошибке после записи новых слотов они **не удаляются**: возможно, это уже
/// единственная копия ключа, под которым лежит база. Сессия закрывается, и следующий вход
/// определяет ключ по самой базе (`open_db_finishing_rekey`).
pub fn rekey(
    state: &AppState,
    password: &SecretString,
    now: DateTime<Utc>,
) -> Result<Zeroizing<String>, AppError> {
    if !state.is_unlocked() {
        return Err(AppError::Locked);
    }
    let store = state.vault();
    let plan = store
        .begin_rekey(password, now)
        .map_err(|e| vault_err(&store, now, e))?;
    let key = Zeroizing::new(*plan.new_dek.as_bytes());
    if let Err(err) = state.with_session_sync(|db| db.rekey(&key).map_err(AppError::from)) {
        state.lock();
        return Err(err);
    }
    if let Err(err) = store.finish_rekey() {
        state.lock();
        return Err(vault_err(&store, now, err));
    }
    state.allow_recovery_save(true);
    Ok(plan.recovery_code.display())
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
mod tests {
    use chrono::{Duration, TimeZone};
    use tempfile::TempDir;

    use super::*;
    use crate::state::AppPaths;

    const FAST: KdfParams = KdfParams {
        m_kib: 8,
        t: 1,
        p: 1,
    };
    const PASSWORD: &str = "correct horse battery";

    fn pw(s: &str) -> SecretString {
        SecretString::from(s)
    }

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
    }

    /// Каталог объявлен первым, чтобы удаляться после состояния (на Windows
    /// открытая БД держит файл).
    fn fresh() -> (TempDir, AppState) {
        let dir = TempDir::new().unwrap();
        let state = AppState::new(AppPaths::new(dir.path().to_path_buf()));
        (dir, state)
    }

    fn created() -> (TempDir, AppState, Zeroizing<String>) {
        let (dir, state) = fresh();
        let code = create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
        (dir, state, code)
    }

    fn categories(state: &AppState) -> i64 {
        tauri::async_runtime::block_on(state.with_session(|db| {
            db.conn()
                .query_row("SELECT count(*) FROM categories", [], |r| r.get(0))
                .map_err(|e| AppError::internal("test", &e))
        }))
        .unwrap()
    }

    #[test]
    fn status_before_and_after_create() {
        let (_dir, state) = fresh();
        let before = status(&state, t0()).unwrap();
        assert!(!before.exists);
        assert!(before.locked);

        create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
        let after = status(&state, t0()).unwrap();
        assert!(after.exists);
        assert!(!after.locked);
        assert_eq!(after.retry_after_ms, 0);
    }

    #[test]
    fn create_opens_a_seeded_session_and_allows_saving_the_code_once() {
        let (_dir, state, code) = created();
        assert_eq!(categories(&state), 17);
        assert!(state.recovery_save_allowed());
        assert_eq!(code.split('-').count(), 7);
    }

    #[test]
    fn create_twice_is_a_conflict() {
        let (_dir, state, _code) = created();
        let err = create(&state, &pw("another long password"), t0(), Some(FAST)).unwrap_err();
        assert!(matches!(err, AppError::Conflict { .. }));
    }

    #[test]
    fn create_rejects_a_short_password_without_leaving_files() {
        let (dir, state) = fresh();
        let err = create(&state, &pw("short"), t0(), Some(FAST)).unwrap_err();
        assert!(matches!(err, AppError::Validation { .. }));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn create_refuses_to_adopt_an_orphan_database() {
        let (dir, state) = fresh();
        fs::write(dir.path().join("budget.db"), b"orphan").unwrap();
        let err = create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap_err();
        assert!(matches!(err, AppError::Conflict { .. }));
        assert!(
            dir.path().join("budget.db").exists(),
            "orphan is not deleted"
        );
    }

    #[test]
    fn lock_then_unlock_restores_access_to_the_same_data() {
        let (_dir, state, _code) = created();
        assert!(state.lock());
        let locked = tauri::async_runtime::block_on(state.with_session(|_| Ok(())));
        assert!(matches!(locked, Err(AppError::Locked)));

        unlock(&state, &pw(PASSWORD), t0()).unwrap();
        assert_eq!(categories(&state), 17);
        assert!(
            !state.recovery_save_allowed(),
            "lock revokes saving the code"
        );
    }

    #[test]
    fn wrong_password_reports_backoff_after_three_failures() {
        let (_dir, state, _code) = created();
        state.lock();
        for attempt in 1..=3 {
            let err = unlock(&state, &pw("wrong password!"), t0()).unwrap_err();
            let AppError::WrongPassword { retry_after_ms } = err else {
                panic!("expected WrongPassword, got {err:?}");
            };
            assert_eq!(retry_after_ms, if attempt < 3 { 0 } else { 1000 });
        }
        let blocked = unlock(&state, &pw(PASSWORD), t0()).unwrap_err();
        assert!(matches!(
            blocked,
            AppError::TooManyAttempts {
                retry_after_ms: 1000
            }
        ));
        assert_eq!(status(&state, t0()).unwrap().retry_after_ms, 1000);

        unlock(&state, &pw(PASSWORD), t0() + Duration::seconds(2)).unwrap();
    }

    #[test]
    fn unlock_applies_the_autolock_setting() {
        let (_dir, state, _code) = created();
        tauri::async_runtime::block_on(state.with_session(|db| {
            db.conn()
                .execute(
                    "UPDATE settings SET value = '0' WHERE key = 'security.autolock_minutes'",
                    [],
                )
                .map_err(|e| AppError::internal("test", &e))
        }))
        .unwrap();
        state.lock();
        unlock(&state, &pw(PASSWORD), t0()).unwrap();
        let far = crate::idle::Moment {
            monotonic: std::time::Instant::now() + std::time::Duration::from_secs(7 * 24 * 3600),
            wall: std::time::SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600),
        };
        assert!(!state.idle().expired(far), "0 minutes = never");
    }

    #[test]
    fn change_password_swaps_the_secret_but_keeps_the_data() {
        let (_dir, state, _code) = created();
        state.lock();
        let new = pw("a completely new secret");
        let wrong = change_password(&state, &pw("wrong old password"), &new, t0()).unwrap_err();
        assert!(matches!(wrong, AppError::WrongPassword { .. }));

        change_password(&state, &pw(PASSWORD), &new, t0()).unwrap();
        assert!(matches!(
            unlock(&state, &pw(PASSWORD), t0()).unwrap_err(),
            AppError::WrongPassword { .. }
        ));
        unlock(&state, &new, t0()).unwrap();
        assert_eq!(categories(&state), 17);
    }

    #[test]
    fn recovery_code_unlocks_and_sets_a_new_password() {
        let (_dir, state, code) = created();
        state.lock();
        let new = pw("forgotten and replaced");
        unlock_recovery(&state, &pw(&code), &new, t0()).unwrap();
        assert_eq!(categories(&state), 17);

        state.lock();
        unlock(&state, &new, t0()).unwrap();
    }

    #[test]
    fn bad_recovery_code_is_a_validation_error() {
        let (_dir, state, _code) = created();
        state.lock();
        let new = pw("forgotten and replaced");
        for bad in ["nonsense", "AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AA"] {
            let err = unlock_recovery(&state, &pw(bad), &new, t0()).unwrap_err();
            assert!(matches!(err, AppError::Validation { .. }), "{bad}");
        }
        assert!(!state.is_unlocked());
    }

    #[test]
    fn reset_needs_phrase_and_password_then_removes_everything() {
        let (dir, state, _code) = created();
        let phrase_err = reset(&state, &pw(PASSWORD), "нет", t0()).unwrap_err();
        assert!(matches!(phrase_err, AppError::Validation { .. }));
        let pw_err = reset(&state, &pw("wrong password!"), RESET_PHRASE, t0()).unwrap_err();
        assert!(matches!(pw_err, AppError::WrongPassword { .. }));
        assert!(state.is_unlocked(), "nothing happened yet");

        reset(&state, &pw(PASSWORD), "  Удалить ВСЕ данные ", t0()).unwrap();
        assert!(!state.is_unlocked());
        // Остаётся только файл оболочки (тема, масштаб): данных и ключей в нём нет.
        let left: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|name| name != "ui-prefs.json")
            .collect();
        assert!(left.is_empty(), "{left:?}");
        assert!(!status(&state, t0()).unwrap().exists);
    }

    #[test]
    fn recovery_file_is_written_once_and_only_when_allowed() {
        let (dir, state, code) = created();
        let target = dir.path().join("code.txt");
        let target2 = target.clone();

        assert!(
            !save_recovery_code(&state, &pw(&code), || None).unwrap(),
            "dialog cancelled"
        );
        assert!(state.recovery_save_allowed(), "cancel keeps the right");

        assert!(save_recovery_code(&state, &pw(&code), move || Some(target2)).unwrap());
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            format!("{}\n", code.as_str())
        );

        let again = save_recovery_code(&state, &pw(&code), || Some(target.clone())).unwrap_err();
        assert!(matches!(again, AppError::Conflict { .. }));
    }

    #[test]
    fn save_is_refused_when_the_session_never_issued_a_code() {
        let (_dir, state) = fresh();
        let err = save_recovery_code(&state, &pw("x"), || None).unwrap_err();
        assert!(matches!(err, AppError::Conflict { .. }));
    }
}

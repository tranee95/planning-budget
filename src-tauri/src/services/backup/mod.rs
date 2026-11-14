//! Зашифрованные резервные копии и открытие копии как отдельного хранилища
//!.
//!
//! Копия — это `vault.json` и `budget.db` хранилища на момент создания: оба файла уже
//! зашифрованы, поэтому копия безопасна на любом диске. Открывается она паролем, который был
//! у хранилища в момент копирования. Импорт копии ничего не заменяет: она становится новым
//! хранилищем рядом с остальными.

use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use planning_budget_storage::Db;
use planning_budget_vault::{VaultStore, new_vault_id};
use secrecy::SecretString;

use crate::AppError;
use crate::dto::{BackupItemDto, BackupStatusDto};
use crate::services::vault::vault_err;
use crate::state::AppState;
use crate::workspace::{self, MAIN_ID, VaultEntry, Workspace, clean_label};

mod format;

const EXT: &str = "budgetbak";
const STAMP: &str = "%Y%m%d-%H%M%S";
/// Сколько последних копий одного хранилища остаётся после создания новой.
pub const KEEP: usize = 7;

fn io(key: &str) -> AppError {
    AppError::Io {
        message_key: format!("errors.io.{key}"),
    }
}

/// Каталог копий: выбранный пользователем или `backups/` в каталоге данных. Создаётся по требованию.
fn dir(state: &AppState) -> (PathBuf, bool) {
    match workspace::load(&state.paths().workspace()).backup_dir {
        Some(custom) => (custom, false),
        None => (state.paths().default_backup_dir(), true),
    }
}

/// Имена файлов начинаются с короткого идентификатора хранилища, чтобы копии разных хранилищ
/// в одной папке не путались и ротация не трогала чужие.
fn prefix(id: &str) -> String {
    if id == MAIN_ID {
        MAIN_ID.to_owned()
    } else {
        id.chars().filter(char::is_ascii_hexdigit).take(8).collect()
    }
}

fn parse_name(name: &str, prefix: &str) -> Option<NaiveDateTime> {
    let stem = name.strip_suffix(&format!(".{EXT}"))?;
    let stamp = stem.strip_prefix(prefix)?.strip_prefix('-')?;
    NaiveDateTime::parse_from_str(stamp, STAMP).ok()
}

fn active_id(state: &AppState) -> String {
    workspace::load(&state.paths().workspace()).active
}

fn items_in(dir: &Path, prefix: &str) -> Vec<(NaiveDateTime, BackupItemDto)> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<_> = read
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let at = parse_name(&name, prefix)?;
            let size = entry.metadata().ok()?.len();
            Some((
                at,
                BackupItemDto {
                    file_name: name,
                    created_at: at
                        .and_utc()
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    size_bytes: i64::try_from(size).unwrap_or(i64::MAX),
                },
            ))
        })
        .collect();
    found.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    found
}

/// Каталог и копии активного хранилища, новые первыми.
#[must_use]
pub fn status(state: &AppState) -> BackupStatusDto {
    let (dir, is_default) = dir(state);
    let items = items_in(&dir, &prefix(&active_id(state)))
        .into_iter()
        .map(|(_, item)| item)
        .collect();
    BackupStatusDto {
        dir: dir.to_string_lossy().into_owned(),
        dir_is_default: is_default,
        items,
    }
}

/// Запоминает каталог, выбранный в диалоге Rust. Права на запись заранее не проверяются: пробный
/// файл в чужом каталоге остался бы мусором при сбое, а отказ
/// покажет первая же копия (`errors.io.backup_dir`).
///
/// # Errors
/// `Io`, если путь не каталог.
pub fn set_dir(state: &AppState, chosen: PathBuf) -> Result<(), AppError> {
    if !chosen.is_dir() {
        return Err(io("backup_dir"));
    }
    workspace::update(&state.paths().workspace(), |ws| {
        ws.backup_dir = Some(chosen);
        Ok(())
    })
}

/// Возвращает каталог по умолчанию. Уже созданные копии остаются на прежнем месте.
///
/// # Errors
/// `Io` при записи реестра.
pub fn reset_dir(state: &AppState) -> Result<(), AppError> {
    workspace::update(&state.paths().workspace(), |ws| {
        ws.backup_dir = None;
        Ok(())
    })
}

/// Создаёт копию активного хранилища. Нужна открытая сессия: база сбрасывает WAL в файл
/// и остаётся заблокированной, пока файл копируется.
///
/// # Errors
/// `Locked`, `Io` при записи, ошибка базы.
pub fn create(state: &AppState, now: DateTime<Utc>) -> Result<BackupItemDto, AppError> {
    let (dir, _) = dir(state);
    fs::create_dir_all(&dir).map_err(|_| io("backup_dir"))?;
    let id = active_id(state);
    let prefix = prefix(&id);
    let name = format!("{prefix}-{}.{EXT}", now.format(STAMP));
    let dest = dir.join(&name);
    if dest.exists() {
        return Err(AppError::Conflict {
            message_key: "errors.backup.too_fast".into(),
        });
    }
    let vault_dir = state.paths().active_dir();
    let manifest = format::Manifest::new(
        id.clone(),
        now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    );
    {
        // Сначала сейф, потом база: тот же порядок, что у остальных операций над ключами.
        let _vault = state.vault();
        state.with_session_sync(|db| {
            db.checkpoint()?;
            let vault_json = fs::read(vault_dir.join("vault.json")).map_err(|_| io("vault"))?;
            format::write(&dest, &manifest, &vault_json, &state.paths().db())
        })?;
    }
    rotate(&dir, &prefix);
    let size = fs::metadata(&dest).map_or(0, |m| m.len());
    Ok(BackupItemDto {
        file_name: name,
        created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        size_bytes: i64::try_from(size).unwrap_or(i64::MAX),
    })
}

/// Оставляет [`KEEP`] последних копий хранилища. Ошибка удаления не мешает: лишняя копия
/// уберётся при следующем создании.
fn rotate(dir: &Path, prefix: &str) {
    for (_, old) in items_in(dir, prefix).into_iter().skip(KEEP) {
        let _ = fs::remove_file(dir.join(old.file_name));
    }
}

/// Пора ли создавать автоматическую копию: прошло не меньше `interval_days` календарных дней
/// (по дате UTC в имени файла) с последней копии этого хранилища или копий ещё нет.
#[must_use]
pub fn is_due(last: Option<NaiveDate>, today: NaiveDate, interval_days: u8) -> bool {
    last.is_none_or(|last| (today - last).num_days() >= i64::from(interval_days.max(1)))
}

/// Автоматическая копия после входа: по расписанию из настроек хранилища. Сбой копии не мешает
/// работе: в журнал идёт только факт.
pub fn run_auto(state: &AppState, now: DateTime<Utc>) {
    let schedule = state.with_session_sync(|db| crate::services::settings::backup_schedule(db));
    let Ok((true, interval_days)) = schedule else {
        return;
    };
    let (dir, _) = dir(state);
    let last = items_in(&dir, &prefix(&active_id(state)))
        .first()
        .map(|(at, _)| at.date());
    if !is_due(last, now.date_naive(), interval_days) {
        return;
    }
    if create(state, now).is_err() {
        tracing::warn!("automatic backup failed");
    }
}

/// Простое имя файла копии: один компонент пути, без разделителей (`/` и `\` отвергаются на любой
/// платформе: на Linux `\` не разделитель, но копию с таким именем нельзя открыть на Windows) и `:`. Двоеточие закрывает букву
/// диска без корня (`C:x`: `Path::join` на Windows заменил бы каталог копий) и альтернативные
/// потоки данных NTFS (`x:y`).
fn is_plain_name(name: &str) -> bool {
    let mut parts = Path::new(name).components();
    let one_component = matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(n)), None) if n == name
    );
    one_component && !name.contains([':', '\\', '/']) && name.ends_with(&format!(".{EXT}"))
}

/// Копия из списка каталога: имя проверяется, путь из интерфейса не принимается.
///
/// # Errors
/// `Validation` для имени не из каталога, затем ошибки [`import`].
pub fn import_named(
    state: &AppState,
    file_name: &str,
    password: &SecretString,
    label: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if !is_plain_name(file_name) {
        return Err(AppError::invalid_field("backup_name_invalid", "fileName"));
    }
    let (dir, _) = dir(state);
    import(state, &dir.join(file_name), password, label, now)
}

/// Открывает копию как новое хранилище и делает его активным. Порядок: распаковать во
/// временную папку → войти паролем копии → открыть базу и проверить целостность → переименовать
/// папку → записать в реестр. Любой сбой удаляет временную папку, остальные хранилища не меняются.
///
/// # Errors
/// `Validation` для названия или файла не того вида, `WrongPassword`, `Io`, ошибка базы.
pub fn import(
    state: &AppState,
    src: &Path,
    password: &SecretString,
    label: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let label = clean_label(label)?;
    let id = new_vault_id().map_err(|e| AppError::internal("vault id", &e))?;
    let vaults_root = state.paths().data_dir().join("vaults");
    let tmp = vaults_root.join(format!("restore.tmp.{id}"));
    fs::create_dir_all(&tmp).map_err(|_| io("workspace"))?;
    let result = import_into(state, src, password, &tmp, &id, label, now);
    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
    }
    result
}

fn import_into(
    state: &AppState,
    src: &Path,
    password: &SecretString,
    tmp: &Path,
    id: &str,
    label: String,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    format::extract(src, tmp)?;
    let copy = VaultStore::new(tmp.to_path_buf());
    // Попытки входа в исходном хранилище к копии не относятся: иначе верный пароль уткнулся бы
    // в чужую задержку.
    copy.clear_failed_attempts()
        .map_err(|e| vault_err(&copy, now, e))?;
    let dek = copy
        .unlock(password, now)
        .map_err(|e| vault_err(&copy, now, e))?;
    // База открывается и проверяется до переименования: плохая копия не станет хранилищем.
    let db = Db::open(&tmp.join("budget.db"), dek.as_bytes())?;
    db.integrity_check()?;
    drop(db);
    let dir = Workspace::vault_dir(state.paths().data_dir(), id);
    fs::rename(tmp, &dir).map_err(|_| io("workspace"))?;
    let registered = workspace::update(&state.paths().workspace(), |ws| {
        ws.vaults.push(VaultEntry {
            id: id.to_owned(),
            label,
        });
        id.clone_into(&mut ws.active);
        Ok(())
    });
    if let Err(err) = registered {
        let _ = fs::remove_dir_all(&dir);
        return Err(err);
    }
    state.activate(dir);
    Ok(())
}

#[cfg(test)]
mod tests;

//! Часть чек-листа безопасности, которую можно проверить только на уровне приложения:
//! перевыпуск ключа на живой БД, отсутствие данных в логах и отсутствие лишних файлов.

use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use budget_storage::Db;
use budget_vault::KdfParams;
use chrono::{DateTime, TimeZone, Utc};
use secrecy::SecretString;
use tempfile::TempDir;
use tracing_subscriber::fmt::MakeWriter;

use super::vault;
use crate::AppError;
use crate::state::{AppPaths, AppState};

const FAST: KdfParams = KdfParams {
    m_kib: 8,
    t: 1,
    p: 1,
};
const PASSWORD: &str = "correct horse battery";
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../testdata/seed-2026.json");

/// Всё, что разрешает держать в каталоге данных.
const ALLOWED: &[&str] = &[
    "vault.json",
    "budget.db",
    "budget.db-wal",
    "budget.db-shm",
    "ui-prefs.json",
    "logs",
    "backups",
];

fn pw(s: &str) -> SecretString {
    SecretString::from(s)
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

/// Каталог объявлен первым: он удаляется после состояния (на Windows открытая БД держит файл).
fn fresh() -> (TempDir, AppState) {
    let dir = TempDir::new().unwrap();
    let state = AppState::new(AppPaths::new(dir.path().to_path_buf()));
    (dir, state)
}

fn assert_only_documented_files(dir: &Path, step: &str) {
    let names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let stray: Vec<&String> = names
        .iter()
        .filter(|n| !ALLOWED.contains(&n.as_str()))
        .collect();
    assert!(stray.is_empty(), "после «{step}» лишние файлы: {stray:?}");
}

fn setting(state: &AppState, key: &'static str) -> Result<Option<String>, AppError> {
    state.with_session_sync(|db| db.setting(key).map_err(AppError::from))
}

#[test]
fn rekey_old_vault_json_with_old_password_does_not_open_the_current_database() {
    let (dir, state) = fresh();
    let old_code = vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    state
        .with_session_sync(|db| {
            db.conn()
                .execute(
                    "UPDATE settings SET value = '15' WHERE key = 'security.autolock_minutes'",
                    [],
                )
                .map_err(|e| AppError::internal("test", &e))
        })
        .unwrap();
    state.lock();
    let vault_json = dir.path().join("vault.json");
    let old_json = fs::read(&vault_json).unwrap();
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();

    let new_code = vault::rekey(&state, &pw(PASSWORD), t0()).unwrap();
    assert_ne!(*old_code, *new_code);
    assert_eq!(
        setting(&state, "security.autolock_minutes")
            .unwrap()
            .as_deref(),
        Some("15")
    );
    assert!(
        state.recovery_save_allowed(),
        "new code may be saved to a file once"
    );
    state.lock();
    assert_only_documented_files(dir.path(), "перевыпуск ключа");

    // Новый файл ключей открывает текущую базу, данные на месте.
    let new_json = fs::read(&vault_json).unwrap();
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert_eq!(
        setting(&state, "security.autolock_minutes")
            .unwrap()
            .as_deref(),
        Some("15")
    );
    state.lock();

    // Старый vault.json + старый пароль разворачивают старый DEK, а база уже под новым.
    fs::write(&vault_json, &old_json).unwrap();
    let err = vault::unlock(&state, &pw(PASSWORD), t0()).unwrap_err();
    assert!(matches!(err, AppError::Internal { .. }), "{err:?}");
    assert!(!state.is_unlocked());

    // Старый recovery-код тоже не помогает.
    let err = vault::unlock_recovery(&state, &pw(&old_code), &pw("another long password"), t0())
        .unwrap_err();
    assert!(matches!(err, AppError::Internal { .. }), "{err:?}");
    assert!(!state.is_unlocked());

    // Новый файл ключей и новый код работают, пароль можно сменить через код.
    fs::write(&vault_json, new_json).unwrap();
    vault::unlock_recovery(&state, &pw(&new_code), &pw("another long password"), t0()).unwrap();
    assert_eq!(
        setting(&state, "security.autolock_minutes")
            .unwrap()
            .as_deref(),
        Some("15")
    );
    assert_only_documented_files(dir.path(), "вход по новому коду");
}

#[test]
fn rekey_requires_an_open_session_and_the_right_password() {
    let (dir, state) = fresh();
    assert!(matches!(
        vault::rekey(&state, &pw(PASSWORD), t0()).unwrap_err(),
        AppError::Locked
    ));

    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    let before = fs::read(dir.path().join("vault.json")).unwrap();
    let err = vault::rekey(&state, &pw("wrong password!"), t0()).unwrap_err();
    assert!(matches!(err, AppError::WrongPassword { .. }));
    assert!(state.is_unlocked());
    // Неудачная попытка меняет только счётчик, слоты остаются прежними.
    let after: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("vault.json")).unwrap()).unwrap();
    let before: serde_json::Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(after["pw"], before["pw"]);
    assert_eq!(after["rc"], before["rc"]);
    assert!(after.get("next").is_none());
}

#[test]
fn interrupted_rekey_after_database_was_rewritten_is_finished_at_next_unlock() {
    let (dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    let store = budget_vault::VaultStore::new(state.paths().data_dir().to_path_buf());

    // Процесс «упал» после PRAGMA rekey, но до замены слотов.
    let plan = store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    let key = *plan.new_dek.as_bytes();
    state
        .with_session_sync(|db| db.rekey(&key).map_err(AppError::from))
        .unwrap();
    state.lock();
    assert!(store.rekey_pending().unwrap());

    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert!(!store.rekey_pending().unwrap(), "слоты доведены до конца");
    assert_eq!(
        setting(&state, "security.autolock_minutes")
            .unwrap()
            .as_deref(),
        Some("5")
    );
    state.lock();
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert_only_documented_files(dir.path(), "доводка перевыпуска");
}

#[test]
fn interrupted_rekey_before_database_was_touched_is_discarded_at_next_unlock() {
    let (_dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    let store = budget_vault::VaultStore::new(state.paths().data_dir().to_path_buf());
    store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    state.lock();

    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert!(!store.rekey_pending().unwrap());
    assert_eq!(
        setting(&state, "security.autolock_minutes")
            .unwrap()
            .as_deref(),
        Some("5")
    );
}

#[test]
fn data_directory_holds_only_documented_files_through_the_whole_lifecycle() {
    let (dir, state) = fresh();
    let new_password = pw("a completely new secret");
    let check = |step: &str| assert_only_documented_files(dir.path(), step);

    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    check("создание");
    state.lock();
    check("блокировка");
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    check("вход");
    vault::change_password(&state, &pw(PASSWORD), &new_password, t0()).unwrap();
    check("смена пароля");
    vault::rekey(&state, &new_password, t0()).unwrap();
    check("перевыпуск ключа");
    state.lock();
    assert_only_documented_files(dir.path(), "блокировка в конце");

    vault::unlock(&state, &pw("a completely new secret"), t0()).unwrap();
    vault::reset(
        &state,
        &pw("a completely new secret"),
        vault::RESET_PHRASE,
        t0(),
    )
    .unwrap();
    assert_only_documented_files(dir.path(), "сброс");
    // Остаться может только файл оболочки (тема, масштаб): данных бюджета и ключей в нём нет.
    let left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| name != "ui-prefs.json")
        .collect();
    assert!(
        left.is_empty(),
        "после сброса не остаётся ни базы, ни ключей: {left:?}"
    );
    assert_eq!(
        crate::prefs::load(&state.paths().prefs()).autolock_minutes,
        None,
        "копия порога очищена"
    );
}

/// Приёмник журнала для проверки: всё, что `tracing` захотел записать, попадает в буфер.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

struct CapturedWriter(Captured);

impl io::Write for CapturedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (self.0)
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = CapturedWriter;

    fn make_writer(&'a self) -> Self::Writer {
        CapturedWriter(self.clone())
    }
}

/// Один подписчик на весь тестовый процесс. Локальный (`with_default`) не годится:
/// параллельный тест, первым дошедший до `tracing::error!` без подписчика, кэширует
/// «никто не слушает», и записи этого теста пропадают.
fn global_capture() -> &'static Captured {
    static CAPTURE: OnceLock<Captured> = OnceLock::new();
    CAPTURE.get_or_init(|| {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(captured.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
        captured
    })
}

/// Названия и суммы из реальной фикстуры: то, чего в логах быть не должно.
fn fixture_secrets() -> Vec<String> {
    let json: serde_json::Value = serde_json::from_slice(&fs::read(FIXTURE).unwrap()).unwrap();
    let mut secrets = Vec::new();
    for tx in json["transactions"].as_array().unwrap() {
        secrets.push(tx["title"].as_str().unwrap().to_owned());
        let kopecks = tx["amount"].as_i64().unwrap();
        secrets.push(kopecks.to_string());
        secrets.push((kopecks / 100).to_string());
    }
    for inc in json["incomes"].as_array().unwrap() {
        secrets.push(inc["source"].as_str().unwrap().to_owned());
        secrets.push(inc["amount"].as_i64().unwrap().to_string());
    }
    secrets.retain(|s| s.chars().count() >= 4);
    secrets.sort();
    secrets.dedup();
    secrets
}

#[test]
fn logs_of_a_full_session_contain_no_amounts_titles_passwords_or_keys() {
    let captured = global_capture();

    let secrets = fixture_secrets();
    assert!(secrets.len() > 100, "фикстура прочитана");
    let passwords = [
        "correct horse battery",
        "a completely new secret",
        "wrong password!",
    ];

    let mut recovery_codes = Vec::new();
    {
        let (dir, state) = fresh();
        recovery_codes.push(
            vault::create(&state, &pw(PASSWORD), t0(), Some(FAST))
                .unwrap()
                .to_string(),
        );

        // Данные бюджета из фикстуры лежат в базе, пока идут операции с сейфом.
        let json: serde_json::Value = serde_json::from_slice(&fs::read(FIXTURE).unwrap()).unwrap();
        state
            .with_session_sync(|db| {
                let cat: i64 = db
                    .conn()
                    .query_row("SELECT id FROM categories LIMIT 1", [], |r| r.get(0))
                    .map_err(|e| AppError::internal("test", &e))?;
                for tx in json["transactions"].as_array().unwrap() {
                    db.conn()
                        .execute(
                            "INSERT INTO transactions (month, category_id, title, amount, status, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, 'paid', 'now', 'now')",
                            rusqlite_params(tx, cat),
                        )
                        .map_err(|e| AppError::internal("test", &e))?;
                }
                Ok(())
            })
            .unwrap();

        // Ошибочные пути тоже пишут в журнал: неверный пароль, перевыпуск, ограничение БД.
        state.lock();
        let _ = vault::unlock(&state, &pw("wrong password!"), t0());
        vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
        let _ = state.with_session_sync(|db| {
            db.conn()
                .execute(
                    "INSERT INTO transactions (month, category_id, title, amount, status, created_at, updated_at)
                     VALUES ('2026-09', 1, 'Лента', 0, 'paid', 'now', 'now')",
                    [],
                )
                .map_err(|e| AppError::internal("storage", &e))
        });
        recovery_codes.push(
            vault::rekey(&state, &pw(PASSWORD), t0())
                .unwrap()
                .to_string(),
        );
        vault::change_password(&state, &pw(PASSWORD), &pw("a completely new secret"), t0())
            .unwrap();
        state.lock();

        // Старый файл ключей даёт Internal(KeyRejected) и запись в журнале.
        let vault_json = dir.path().join("vault.json");
        let current = fs::read(&vault_json).unwrap();
        let _ = vault::unlock(&state, &pw("a completely new secret"), t0());
        fs::write(&vault_json, current).unwrap();
        drop(state);
    }

    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(
        log.contains("internal error"),
        "журнал не пуст, проверка осмысленна:\n{log}"
    );
    // Названия и суммы — только как отдельные слова: короткое число вроде «3500» иначе
    // совпадало бы с цифрами внутри временной метки или hex-идентификатора ошибки.
    for needle in &secrets {
        assert!(!contains_word(&log, needle), "в журнале найдено «{needle}»");
    }
    // Пароли и коды длинные и уникальные: достаточно подстроки.
    for needle in passwords
        .iter()
        .copied()
        .chain(recovery_codes.iter().map(String::as_str))
    {
        assert!(!log.contains(needle), "в журнале найдено «{needle}»");
    }
}

/// Есть ли `needle` в `haystack` как отдельное слово (по краям не буква и не цифра).
fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(at, _)| {
        let before = haystack[..at].chars().next_back();
        let after = haystack[at + needle.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

#[test]
fn contains_word_ignores_digits_inside_longer_tokens() {
    assert!(contains_word("amount=3500 rub", "3500"));
    assert!(contains_word("title: Лента.", "Лента"));
    assert!(!contains_word("2026-10-01T15:15:47.350012Z", "3500"));
    assert!(!contains_word("id=19a3500-7", "3500"));
    assert!(!contains_word("Лентаа", "Лента"));
}

/// Параметры вставки траты из фикстуры: месяц, категория, название, сумма.
fn rusqlite_params(tx: &serde_json::Value, cat: i64) -> impl rusqlite::Params {
    (
        tx["month"].as_str().unwrap().to_owned(),
        cat,
        tx["title"].as_str().unwrap().to_owned(),
        tx["amount"].as_i64().unwrap(),
    )
}

#[test]
fn database_without_the_right_key_is_unreadable_even_through_the_app_open_path() {
    let (dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    state.lock();
    assert!(Db::open(&state.paths().db(), &[1; 32]).is_err());
    let head = fs::read(dir.path().join("budget.db")).unwrap();
    assert!(!head.starts_with(b"SQLite format 3\0"));
}

// ---- Регрессии по ревью ----

fn raw_store(state: &AppState) -> budget_vault::VaultStore {
    budget_vault::VaultStore::new(state.paths().data_dir().to_path_buf())
}

/// Доводит перевыпуск до состояния «база под новым ключом, слоты не заменены».
fn interrupt_rekey_after_database(state: &AppState, password: &str) {
    let plan = raw_store(state).begin_rekey(&pw(password), t0()).unwrap();
    let key = *plan.new_dek.as_bytes();
    state
        .with_session_sync(|db| db.rekey(&key).map_err(AppError::from))
        .unwrap();
}

#[test]
fn unlock_with_a_missing_database_reports_it_instead_of_creating_an_empty_one() {
    let (dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    state.lock();
    for path in state.paths().db_files() {
        let _ = fs::remove_file(path);
    }

    let err = vault::unlock(&state, &pw(PASSWORD), t0()).unwrap_err();
    assert!(
        matches!(&err, AppError::Io { message_key } if message_key == "errors.vault.database_missing"),
        "{err:?}"
    );
    assert!(!state.is_unlocked());
    assert!(
        !dir.path().join("budget.db").exists(),
        "no empty database was created"
    );
}

#[test]
fn password_change_during_an_unfinished_rekey_does_not_lock_the_user_out() {
    let (_dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    interrupt_rekey_after_database(&state, PASSWORD);

    // Сессия ещё открыта, слоты не заменены — пользователь меняет пароль.
    let new = "a completely new secret";
    vault::change_password(&state, &pw(PASSWORD), &pw(new), t0()).unwrap();
    state.lock();

    vault::unlock(&state, &pw(new), t0()).unwrap();
    assert!(!raw_store(&state).rekey_pending().unwrap());
    assert_eq!(
        setting(&state, "security.autolock_minutes")
            .unwrap()
            .as_deref(),
        Some("5")
    );
}

#[test]
fn second_rekey_is_refused_while_the_first_one_is_unfinished() {
    let (_dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    interrupt_rekey_after_database(&state, PASSWORD);

    let err = vault::rekey(&state, &pw(PASSWORD), t0()).unwrap_err();
    assert!(
        matches!(&err, AppError::Conflict { message_key } if message_key == "errors.vault.rekey_pending"),
        "{err:?}"
    );
    // Отказ не тронул слоты: вход доводит первый перевыпуск до конца.
    state.lock();
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert!(!raw_store(&state).rekey_pending().unwrap());
}

#[test]
fn recovery_code_during_an_unfinished_rekey_works_while_the_database_kept_its_key() {
    let (_dir, state) = fresh();
    let code = vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    raw_store(&state).begin_rekey(&pw(PASSWORD), t0()).unwrap();
    state.lock();

    vault::unlock_recovery(&state, &pw(&code), &pw("another long password"), t0()).unwrap();
    assert!(state.is_unlocked());
    assert!(
        !raw_store(&state).rekey_pending().unwrap(),
        "stale slots are discarded"
    );
}

#[test]
fn recovery_code_after_the_database_was_rekeyed_gives_a_clear_error_not_wrong_password() {
    let (_dir, state) = fresh();
    let code = vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    interrupt_rekey_after_database(&state, PASSWORD);
    state.lock();

    // Остаточный риск старый код разворачивает старый DEK, новый слот под забытым паролем.
    let err =
        vault::unlock_recovery(&state, &pw(&code), &pw("another long password"), t0()).unwrap_err();
    assert!(
        matches!(&err, AppError::Conflict { message_key } if message_key == "errors.vault.rekey_interrupted"),
        "{err:?}"
    );
    assert!(!state.is_unlocked());
}

#[test]
fn attempt_inside_the_backoff_window_is_not_reported_as_a_wrong_password() {
    let (_dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    for _ in 0..3 {
        let _ = vault::change_password(
            &state,
            &pw("wrong password!"),
            &pw("a completely new secret"),
            t0(),
        );
    }
    let err = vault::change_password(&state, &pw(PASSWORD), &pw("a completely new secret"), t0())
        .unwrap_err();
    assert!(
        matches!(
            err,
            AppError::TooManyAttempts {
                retry_after_ms: 1000
            }
        ),
        "{err:?}"
    );
}

#[test]
fn concurrent_status_calls_do_not_disturb_a_vault_write() {
    let (dir, state) = fresh();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    state.lock();
    let state = Arc::new(state);

    let readers: Vec<_> = (0..4)
        .map(|_| {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                for _ in 0..50 {
                    vault::status(&state, t0()).unwrap();
                }
            })
        })
        .collect();
    for _ in 0..20 {
        // Каждая неудача переписывает vault.json (счётчик попыток).
        let _ = vault::unlock(&state, &pw("wrong password!"), t0());
    }
    for reader in readers {
        reader.join().unwrap();
    }

    assert_only_documented_files(dir.path(), "параллельные status и запись");
    vault::unlock(&state, &pw(PASSWORD), t0() + chrono::Duration::seconds(120)).unwrap();
}

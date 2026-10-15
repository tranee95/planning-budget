//! «Команда записи сообщает об изменении данных событием `data-changed`» ( → события,
//! ). От события зависят экраны и кеш расчётов UI: если команда меняет данные и молчит,
//! графики и «Обзор» покажут устаревшие числа до перезапуска.
//!
//! Команды здесь не вызываются (сканер по исходникам): каждая `#[tauri::command]` должна либо
//! читать, либо вызывать `data_changed(…)` (или помощник `changed(…)`), либо входить в список
//! команд, которым событие не нужно, — с причиной.

#![allow(
    clippy::unwrap_used,
    reason = "вспомогательные функции интеграционных тестов лежат вне #[test]"
)]

use std::fs;
use std::path::Path;

const MARKER: &str = "#[tauri::command]";

/// Чтение и расчёт без записи: событие не нужно. Имя оканчивается одним из слов или входит в
/// `READ_EXACT`.
const READ_SUFFIXES: &[&str] = &[
    "list", "get", "month", "year", "series", "run", "run_many", "check", "progress", "overview",
    "preview", "search", "suggest", "usage", "defaults", "history", "version", "ping",
];

const READ_EXACT: &[&str] = &[
    "app_version",
    "vault_status",
    "query_parse",
    "search",
    "system_dark",
    "open_data_dir",
    "debt_schedule_preview",
    "plan_preview",
    "update_check",
];

/// Команды записи без `data-changed`, и почему он не нужен.
const NO_DATA_EVENT: &[(&str, &str)] = &[
    // Сейф: блокировка и сброс сообщают `VaultLocked`; пароль, ключи и recovery-код данных бюджета не меняют.
    (
        "vault_retry_migration",
        "миграция при открытии сейфа, экран ещё не показан",
    ),
    ("vault_create", "новый сейф: данных ещё нет"),
    (
        "vault_unlock",
        "после разблокировки экраны загружаются заново",
    ),
    (
        "vault_unlock_recovery",
        "после разблокировки экраны загружаются заново",
    ),
    ("vault_change_password", "пароль не входит в расчёты"),
    ("vault_rekey", "сообщает VaultLocked через notify_locked"),
    ("vault_reset", "сообщает VaultLocked через notify_locked"),
    ("vault_lock", "сообщает VaultLocked через notify_locked"),
    (
        "vault_save_recovery_code",
        "флаг показа кода, не данные бюджета",
    ),
    // Пишет в журнал приложения, данных бюджета не меняет.
    ("client_error_log", "только запись в журнал"),
    // Настройки интерфейса и фильтры не входят в расчёты.
    ("prefs_set", "тема, язык, масштаб: не входят в расчёты"),
    ("filters_save", "сохранённые фильтры не входят в расчёты"),
    ("filters_delete", "сохранённые фильтры не входят в расчёты"),
    // Раскладка и состав дашбордов: ключ кеша графика содержит всю спецификацию, расчёт не меняется.
    ("dashboards_create", "состав дашбордов, не данные расчётов"),
    (
        "dashboards_create_default",
        "состав дашбордов, не данные расчётов",
    ),
    ("dashboards_rename", "состав дашбордов, не данные расчётов"),
    ("dashboards_delete", "состав дашбордов, не данные расчётов"),
    ("charts_create", "спецификация в ключе кеша"),
    ("charts_update", "спецификация в ключе кеша"),
    ("charts_delete", "спецификация в ключе кеша"),
    (
        "charts_layout_set",
        "раскладка карточек, не данные расчётов",
    ),
    // Установка обновления закрывает сессию сама.
    (
        "update_install",
        "закрывает сессию и перезапускает приложение",
    ),
    ("activity_ping", "продлевает сессию, данных не меняет"),
];

fn is_read(name: &str) -> bool {
    READ_EXACT.contains(&name)
        || READ_SUFFIXES
            .iter()
            .any(|s| name == *s || name.ends_with(&format!("_{s}")))
}

/// Имена команд записи, которые не вызывают `data_changed`/`changed`/`notify_locked` и не входят в список.
fn silent_commands(source: &str) -> Vec<String> {
    source
        .split(MARKER)
        .skip(1)
        .filter_map(|chunk| {
            let name = chunk
                .split("fn ")
                .nth(1)?
                .split(['(', '<'])
                .next()?
                .trim()
                .to_owned();
            if is_read(&name) || NO_DATA_EVENT.iter().any(|(n, _)| *n == name) {
                return None;
            }
            let code: String = chunk
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            let emits = code.contains("data_changed(")
                || code.contains("changed(&app")
                || code.contains("notify_locked(");
            (!emits).then_some(name)
        })
        .collect()
}

fn command_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, fs::read_to_string(p).unwrap())
        })
        .collect()
}

#[test]
fn every_write_command_reports_data_changed() {
    let sources = command_sources();
    assert!(!sources.is_empty());
    for (file, source) in sources {
        let bad = silent_commands(&source);
        assert!(
            bad.is_empty(),
            "{file}: команды записи без data_changed и не из списка NO_DATA_EVENT: {bad:?}"
        );
    }
}

#[test]
fn scanner_flags_a_write_command_without_the_event() {
    let source = r#"
        #[tauri::command]
        pub async fn tx_list(state: State<'_, AppState>) -> Result<(), AppError> { Ok(()) }

        #[tauri::command]
        pub async fn tx_create(app: AppHandle) -> Result<(), AppError> {
            state.with_session(|db| Ok(())).await?;
            Ok(())
        }

        #[tauri::command]
        pub async fn tx_delete(app: AppHandle) -> Result<(), AppError> {
            data_changed(&app, ChangeScope::Transactions, vec![]);
            Ok(())
        }

        #[tauri::command]
        pub async fn prefs_set() -> Result<(), AppError> { Ok(()) }
    "#;
    assert_eq!(silent_commands(source), ["tx_create"]);
}

#[test]
fn every_exception_has_a_reason_and_is_a_real_command() {
    let all: String = command_sources().into_iter().map(|(_, s)| s).collect();
    for (name, reason) in NO_DATA_EVENT {
        assert!(!reason.is_empty(), "{name}: нет причины");
        assert!(
            all.contains(&format!("fn {name}(")),
            "{name}: в списке исключений, но такой команды нет"
        );
    }
}

#[test]
fn read_names_are_not_mistaken_for_writes() {
    assert!(is_read("summary_month"));
    assert!(is_read("tx_category_usage"));
    assert!(!is_read("tx_set_status"));
    assert!(!is_read("debt_payment_set_status"));
    assert!(!is_read("plan_lock"));
}

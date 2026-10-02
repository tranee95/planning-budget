//! «После `vault_lock` любая команда с данными возвращает `Locked`».
//!
//! Тестовый бинарник с Tauri на Windows не стартует, поэтому команды здесь не вызываются.
//! Гарантия строится из двух частей: `AppState::with_session` без сессии отвечает `Locked`
//! (юнит-тесты в `state.rs`), а этот тест проверяет по исходникам, что каждая
//! `#[tauri::command]` либо ходит в БД через `with_session`, либо входит в список команд,
//! которые работают до разблокировки.

#![allow(
    clippy::unwrap_used,
    reason = "вспомогательные функции интеграционных тестов лежат вне #[test]"
)]

use std::fs;
use std::path::Path;

/// Команды без доступа к данным бюджета: работают до разблокировки.
const WORKS_WITHOUT_SESSION: &[&str] = &[
    "app_version",
    "vault_status",
    "vault_create",
    "vault_unlock",
    "vault_unlock_recovery",
    "vault_change_password",
    "vault_lock",
    // Сам проверяет сессию в сервисе: нужен и пароль, и открытая БД.
    "vault_rekey",
    "vault_reset",
    "vault_save_recovery_code",
    "activity_ping",
    "prefs_get",
    "prefs_set",
    "system_dark",
    "open_data_dir",
];

const MARKER: &str = "#[tauri::command]";

/// Имена команд, в теле которых нет `with_session` и которых нет в списке исключений.
fn unguarded_commands(source: &str) -> Vec<String> {
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
            // Считается только вызов в коде: упоминание в комментарии (в том числе в
            // doc-комментарии следующей команды) защитой не является.
            let code: String = chunk
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            let guarded =
                code.contains(".with_session(") || WORKS_WITHOUT_SESSION.contains(&name.as_str());
            (!guarded).then_some(name)
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
fn every_data_command_goes_through_with_session() {
    let sources = command_sources();
    assert!(!sources.is_empty());
    for (file, source) in sources {
        let bad = unguarded_commands(&source);
        assert!(
            bad.is_empty(),
            "{file}: команды без with_session и не из списка без сессии: {bad:?}"
        );
    }
}

#[test]
fn scanner_flags_a_command_that_skips_with_session() {
    let source = r#"
        #[tauri::command]
        #[specta::specta]
        pub async fn tx_list(state: State<'_, AppState>) -> Result<(), AppError> {
            let db = state.db.lock();
            Ok(())
        }

        #[tauri::command]
        #[specta::specta]
        pub async fn tx_delete(state: State<'_, AppState>) -> Result<(), AppError> {
            state.with_session(|db| Ok(())).await
        }

        #[tauri::command]
        pub async fn vault_status() -> Result<(), AppError> { Ok(()) }
    "#;
    assert_eq!(unguarded_commands(source), ["tx_list"]);
}

#[test]
fn scanner_ignores_with_session_mentioned_only_in_comments() {
    let source = r"
        #[tauri::command]
        pub async fn tx_list(state: State<'_, AppState>) -> Result<(), AppError> {
            // TODO: перевести на with_session
            Ok(())
        }

        /// Ходит в БД через `state.with_session(...)`.
        #[tauri::command]
        pub async fn tx_delete(state: State<'_, AppState>) -> Result<(), AppError> {
            state.with_session(|db| Ok(())).await
        }
    ";
    assert_eq!(unguarded_commands(source), ["tx_list"]);
}

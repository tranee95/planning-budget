// Без консольного окна в релизной сборке на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

/// Служебные режимы запуска без окна. Экспорт биндингов живёт здесь, а не в тесте:
/// тестовые бинарники на Windows не получают манифест Common Controls и не стартуют
/// (`STATUS_ENTRYPOINT_NOT_FOUND`), а основной бинарник получает его от `tauri-build`.
fn cli_mode(flag: &str) -> Option<Result<(), String>> {
    match flag {
        "--self-test" => {
            Some(budget_app_lib::self_test::run().map_err(|e| format!("self-test failed: {e}")))
        }
        "--export-bindings" => Some(
            budget_app_lib::export_bindings(&budget_app_lib::specta_builder())
                .map_err(|e| format!("bindings export failed: {e}")),
        ),
        _ => None,
    }
}

fn main() -> ExitCode {
    if let Some(result) = std::env::args().skip(1).find_map(|a| cli_mode(&a)) {
        return match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                #[allow(
                    clippy::print_stderr,
                    reason = "CI читает причину провала служебного режима из stderr"
                )]
                {
                    eprintln!("{message}");
                }
                ExitCode::FAILURE
            }
        };
    }
    match budget_app_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

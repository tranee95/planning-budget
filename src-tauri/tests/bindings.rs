//! Генерация `ui/src/lib/api/bindings.ts` без окна; CI после этого проверяет `git diff --exit-code`.
//! Запускает основной бинарник с `--export-bindings`: тестовый бинарник с Tauri на Windows не стартует.

use std::process::Command;

#[test]
fn export_bindings() {
    let status = Command::new(env!("CARGO_BIN_EXE_planning-budget-app"))
        .arg("--export-bindings")
        .status()
        .unwrap();
    assert!(status.success());
}

//! Служебные команды без доступа к данным.

use crate::AppError;

/// Версия приложения. Пока единственная команда: нужна, чтобы `AppError` попал в биндинги.
#[tauri::command]
#[specta::specta]
pub async fn app_version() -> Result<String, AppError> {
    Ok(env!("CARGO_PKG_VERSION").to_owned())
}

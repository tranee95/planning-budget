//! Журнал ошибок интерфейса. Работает и до разблокировки; данных бюджета не читает.

use crate::AppError;
use crate::dto::ClientErrorDto;
use crate::services::client_log as service;

/// Записывает ошибку интерфейса (страница «500», необработанное исключение) в журнал приложения.
#[tauri::command]
#[specta::specta]
pub async fn client_error_log(entry: ClientErrorDto) -> Result<(), AppError> {
    service::record(&entry);
    Ok(())
}

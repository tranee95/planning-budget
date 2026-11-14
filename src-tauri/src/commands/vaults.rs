//! Список хранилищ и переключение между ними.
//! Данных бюджета команды не читают, поэтому работают и на экране входа.

use tauri::AppHandle;

use crate::AppError;
use crate::dto::VaultItemDto;
use crate::events::{self, LockReason};
use crate::services::vaults as service;
use crate::state::blocking;

#[tauri::command]
#[specta::specta]
pub async fn vaults_list(app: AppHandle) -> Result<Vec<VaultItemDto>, AppError> {
    blocking(&app, |state| Ok(service::list(state))).await
}

/// Создаёт пустое хранилище и делает его активным: пароль задают на экране входа.
#[tauri::command]
#[specta::specta]
pub async fn vaults_add(app: AppHandle, label: String) -> Result<(), AppError> {
    blocking(&app, move |state| service::add(state, &label)).await?;
    events::notify_locked(&app, LockReason::Manual);
    Ok(())
}

/// Переключает активное хранилище; текущая сессия закрывается.
#[tauri::command]
#[specta::specta]
pub async fn vaults_switch(app: AppHandle, id: String) -> Result<(), AppError> {
    blocking(&app, move |state| service::switch(state, &id)).await?;
    events::notify_locked(&app, LockReason::Manual);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn vaults_rename(app: AppHandle, id: String, label: String) -> Result<(), AppError> {
    blocking(&app, move |state| service::rename(state, &id, &label)).await
}

/// Убирает из списка хранилище, которое так и не создали.
#[tauri::command]
#[specta::specta]
pub async fn vaults_remove(app: AppHandle, id: String) -> Result<(), AppError> {
    blocking(&app, move |state| service::remove_unused(state, &id)).await
}

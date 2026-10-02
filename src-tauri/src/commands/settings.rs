//! Настройки пользователя.

use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{SettingsDto, SettingsPatchDto};
use crate::events::{ChangeScope, data_changed};
use crate::services::settings as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: State<'_, AppState>) -> Result<SettingsDto, AppError> {
    state.with_session(|db| service::get(db)).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatchDto,
) -> Result<SettingsDto, AppError> {
    let dto = state
        .with_session(move |db| service::set(db, patch))
        .await?;
    service::apply_security(&state, dto.autolock_minutes, dto.lock_on_minimize);
    data_changed(&app, ChangeScope::Settings, Vec::new());
    Ok(dto)
}

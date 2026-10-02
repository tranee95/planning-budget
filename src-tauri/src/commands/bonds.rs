//! Облигации.

use chrono::Local;
use tauri::State;

use crate::AppError;
use crate::dto::BondsDto;
use crate::services::bonds as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn bonds_projection(state: State<'_, AppState>, year: u16) -> Result<BondsDto, AppError> {
    state
        .with_session(move |db| service::projection(db, year, Local::now().date_naive()))
        .await
}

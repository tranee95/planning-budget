//! Команды разработчика. Компилируются только в debug-сборке: в релиз не попадают ни сами
//! команды, ни данные фикстуры.

use chrono::Utc;
use tauri::{AppHandle, State};

use crate::AppError;
use crate::events::{ChangeScope, data_changed};
use crate::services::devseed as service;
use crate::state::AppState;

/// Заливает `seed-2026.json` в пустую базу.
#[tauri::command]
#[specta::specta]
pub async fn dev_seed(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .with_session(move |db| service::load(db, Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::All, Vec::new());
    Ok(())
}

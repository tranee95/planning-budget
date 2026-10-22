//! Раздел «Сбережения».

use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{Int53, SavingsProgressDto};
use crate::events::{ChangeScope, data_changed};
use crate::services::savings as service;
use crate::state::AppState;

/// Фиксированный план накопления в месяц с `valid_from` (вместо процента от дохода).
#[tauri::command]
#[specta::specta]
pub async fn savings_fixed_set(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: Int53,
    valid_from: String,
    amount: Int53,
) -> Result<(), AppError> {
    state
        .with_session(move |db| service::fixed_set(db, category_id.0, &valid_from, amount.0))
        .await?;
    data_changed(&app, ChangeScope::Savings, Vec::new());
    Ok(())
}

/// Накопленное за весь период и взносы по месяцам.
#[tauri::command]
#[specta::specta]
pub async fn savings_progress(
    state: State<'_, AppState>,
    month: String,
) -> Result<SavingsProgressDto, AppError> {
    state
        .with_session(move |db| service::progress(db, &month))
        .await
}

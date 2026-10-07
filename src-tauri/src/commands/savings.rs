//! Раздел «Сбережения».

use chrono::Local;
use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{Int53, SavingsOverviewDto, SavingsParamsDto};
use crate::events::{ChangeScope, data_changed};
use crate::services::savings as service;
use crate::state::AppState;

/// Накопления года: факт, прогноз, итоги и готовые графики.
#[tauri::command]
#[specta::specta]
pub async fn savings_overview(
    state: State<'_, AppState>,
    year: u16,
) -> Result<SavingsOverviewDto, AppError> {
    state
        .with_session(move |db| service::overview(db, year, Local::now().date_naive()))
        .await
}

/// Ставка, налог, стартовый баланс и месяц начала накопления.
#[tauri::command]
#[specta::specta]
pub async fn savings_params_set(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: Int53,
    params: SavingsParamsDto,
) -> Result<(), AppError> {
    state
        .with_session(move |db| service::params_set(db, category_id.0, &params))
        .await?;
    data_changed(&app, ChangeScope::Savings, Vec::new());
    Ok(())
}

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

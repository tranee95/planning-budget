//! Сводки месяца и года.

use chrono::Local;
use tauri::State;

use crate::AppError;
use crate::dto::{MonthOverviewDto, MonthSummaryDto, SeriesRangeDto, YearSummaryDto};
use crate::services::summary as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn summary_month(
    state: State<'_, AppState>,
    month: String,
) -> Result<MonthOverviewDto, AppError> {
    state
        .with_session(move |db| service::month(db, &month, Local::now().date_naive()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn summary_series(
    state: State<'_, AppState>,
    month: String,
    range: SeriesRangeDto,
) -> Result<Vec<MonthSummaryDto>, AppError> {
    state
        .with_session(move |db| service::series(db, &month, range))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn summary_year(
    state: State<'_, AppState>,
    year: u16,
) -> Result<YearSummaryDto, AppError> {
    state
        .with_session(move |db| service::year(db, year, Local::now().date_naive()))
        .await
}

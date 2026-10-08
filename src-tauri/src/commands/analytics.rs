//! Графики и дашборды.

use chrono::{Local, Utc};
use tauri::State;

use crate::AppError;
use crate::dto::{
    CardPlacementDto, ChartCardDto, ChartDataDto, ChartRunDto, ChartSpecDto, DashboardDto, Int53,
};
use crate::services::analytics as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn analytics_run(
    state: State<'_, AppState>,
    spec: ChartSpecDto,
) -> Result<ChartDataDto, AppError> {
    state
        .with_session(move |db| service::run(db, &spec, Local::now().date_naive()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn analytics_run_many(
    state: State<'_, AppState>,
    specs: Vec<ChartSpecDto>,
) -> Result<Vec<ChartRunDto>, AppError> {
    state
        .with_session(move |db| service::run_many(db, &specs, Local::now().date_naive()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboards_list(state: State<'_, AppState>) -> Result<Vec<DashboardDto>, AppError> {
    state.with_session(|db| service::dashboards_list(db)).await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboards_create(
    state: State<'_, AppState>,
    name: String,
) -> Result<DashboardDto, AppError> {
    state
        .with_session(move |db| service::dashboard_create(db, &name))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboards_create_default(
    state: State<'_, AppState>,
) -> Result<DashboardDto, AppError> {
    state
        .with_session(|db| service::dashboard_create_default(db, Utc::now()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboards_rename(
    state: State<'_, AppState>,
    id: Int53,
    name: String,
) -> Result<(), AppError> {
    state
        .with_session(move |db| service::dashboard_rename(db, id.0, &name))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn dashboards_delete(state: State<'_, AppState>, id: Int53) -> Result<(), AppError> {
    state
        .with_session(move |db| service::dashboard_delete(db, id.0))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn charts_list(
    state: State<'_, AppState>,
    dashboard_id: Int53,
) -> Result<Vec<ChartCardDto>, AppError> {
    state
        .with_session(move |db| service::charts_list(db, dashboard_id.0))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn charts_create(
    state: State<'_, AppState>,
    dashboard_id: Int53,
    spec: ChartSpecDto,
    w: u8,
    h: u8,
) -> Result<ChartCardDto, AppError> {
    state
        .with_session(move |db| service::chart_create(db, dashboard_id.0, &spec, w, h, Utc::now()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn charts_update(
    state: State<'_, AppState>,
    id: Int53,
    spec: ChartSpecDto,
) -> Result<ChartCardDto, AppError> {
    state
        .with_session(move |db| service::chart_update(db, id.0, &spec, Utc::now()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn charts_delete(state: State<'_, AppState>, id: Int53) -> Result<(), AppError> {
    state
        .with_session(move |db| service::chart_delete(db, id.0))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn charts_layout_set(
    state: State<'_, AppState>,
    dashboard_id: Int53,
    placements: Vec<CardPlacementDto>,
) -> Result<(), AppError> {
    state
        .with_session(move |db| {
            service::charts_layout_set(db, dashboard_id.0, &placements, Utc::now())
        })
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn analytics_check(
    state: State<'_, AppState>,
    specs: Vec<ChartSpecDto>,
) -> Result<Vec<Option<String>>, AppError> {
    state.with_session(move |_| service::check(&specs)).await
}

//! Доходы.

use chrono::Utc;
use planning_budget_core::IncomeId;
use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{IncomeDto, IncomeInput, IncomePatchDto, IncomeUpdateResultDto, Int53};
use crate::events::{ChangeScope, data_changed};
use crate::services::records as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn incomes_list(
    state: State<'_, AppState>,
    month: String,
) -> Result<Vec<IncomeDto>, AppError> {
    state
        .with_session(move |db| service::incomes_list(db, &month))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn incomes_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: IncomeInput,
) -> Result<IncomeDto, AppError> {
    let dto = state
        .with_session(move |db| service::income_create(db, input, Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Incomes, vec![dto.month.clone()]);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn incomes_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
    patch: IncomePatchDto,
) -> Result<IncomeUpdateResultDto, AppError> {
    let dto = state
        .with_session(move |db| service::income_update(db, IncomeId(id.0), patch, Utc::now()))
        .await?;
    let months = service::touched_months(&dto.previous.month, &dto.current.month);
    data_changed(&app, ChangeScope::Incomes, months);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn incomes_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<IncomeDto, AppError> {
    let dto = state
        .with_session(move |db| service::income_delete(db, IncomeId(id.0), Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Incomes, vec![dto.month.clone()]);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn incomes_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<IncomeDto, AppError> {
    let dto = state
        .with_session(move |db| service::income_restore(db, IncomeId(id.0), Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Incomes, vec![dto.month.clone()]);
    Ok(dto)
}

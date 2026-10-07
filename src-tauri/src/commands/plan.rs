//! План месяца.

use chrono::Utc;
use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{MonthPlanDto, PlanWizardInputDto};
use crate::events::{ChangeScope, data_changed};
use crate::services::plan as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn plan_month(
    state: State<'_, AppState>,
    month: String,
) -> Result<MonthPlanDto, AppError> {
    state
        .with_session(move |db| service::month(db, &month))
        .await
}

/// «План готов»: фиксирует плановые суммы месяца.
#[tauri::command]
#[specta::specta]
pub async fn plan_lock(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
) -> Result<MonthPlanDto, AppError> {
    let dto = state
        .with_session(move |db| service::lock(db, &month, Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Plan, vec![dto.month.clone()]);
    Ok(dto)
}

/// Снимает фиксацию плана.
#[tauri::command]
#[specta::specta]
pub async fn plan_unlock(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
) -> Result<MonthPlanDto, AppError> {
    let dto = state
        .with_session(move |db| service::unlock(db, &month))
        .await?;
    data_changed(&app, ChangeScope::Plan, vec![dto.month.clone()]);
    Ok(dto)
}

/// «Скопировать план из прошлого месяца» в `month`.
#[tauri::command]
#[specta::specta]
pub async fn plan_copy_from_previous(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
) -> Result<MonthPlanDto, AppError> {
    let dto = state
        .with_session(move |db| service::copy_from_previous(db, &month, Utc::now()))
        .await?;
    // Копирование создаёт траты: меняются и записи месяца, и его план.
    data_changed(&app, ChangeScope::Plan, vec![dto.month.clone()]);
    data_changed(&app, ChangeScope::Transactions, vec![dto.month.clone()]);
    Ok(dto)
}

/// Предпросмотр мастера первого месяца: «Не распределено» по введённому, без записи.
#[tauri::command]
#[specta::specta]
pub async fn plan_preview(
    state: State<'_, AppState>,
    month: String,
    input: PlanWizardInputDto,
) -> Result<MonthPlanDto, AppError> {
    state
        .with_session(move |db| service::preview(db, &month, &input))
        .await
}

/// Мастер первого месяца: доходы, плановые траты и план накоплений, затем «План готов».
#[tauri::command]
#[specta::specta]
pub async fn plan_wizard_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
    input: PlanWizardInputDto,
) -> Result<MonthPlanDto, AppError> {
    let dto = state
        .with_session(move |db| service::wizard_apply(db, &month, &input, Utc::now()))
        .await?;
    let months = vec![dto.month.clone()];
    data_changed(&app, ChangeScope::Plan, months.clone());
    data_changed(&app, ChangeScope::Transactions, months.clone());
    data_changed(&app, ChangeScope::Incomes, months);
    data_changed(&app, ChangeScope::Savings, Vec::new());
    Ok(dto)
}

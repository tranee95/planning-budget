//! План месяца.

use crate::clock::Now;
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
        .with_session(move |db| service::lock(db, &month, Now::capture().utc))
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
        .with_session(move |db| service::copy_from_previous(db, &month, Now::capture().utc))
        .await?;
    // Копирование создаёт траты: меняются и записи месяца, и его план.
    data_changed(&app, ChangeScope::Plan, vec![dto.month.clone()]);
    data_changed(&app, ChangeScope::Transactions, vec![dto.month.clone()]);
    Ok(dto)
}

/// Автоперенос плана из последнего месяца с данными в новый (один раз на месяц). Возвращает число
/// созданных строк; события об изменении уходят, только если что-то создано.
#[tauri::command]
#[specta::specta]
pub async fn plan_carry_forward(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
) -> Result<u32, AppError> {
    let target = month.clone();
    let created = state
        .with_session(move |db| service::carry_forward(db, &month, &Now::capture()))
        .await?;
    if created > 0 {
        data_changed(&app, ChangeScope::Plan, vec![target.clone()]);
        data_changed(&app, ChangeScope::Transactions, vec![target]);
    }
    Ok(created)
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
        .with_session(move |db| service::wizard_apply(db, &month, &input, Now::capture().utc))
        .await?;
    let months = vec![dto.month.clone()];
    data_changed(&app, ChangeScope::Plan, months.clone());
    data_changed(&app, ChangeScope::Transactions, months.clone());
    data_changed(&app, ChangeScope::Incomes, months);
    data_changed(&app, ChangeScope::Savings, Vec::new());
    Ok(dto)
}

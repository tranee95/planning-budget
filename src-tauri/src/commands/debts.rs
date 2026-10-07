//! Долги.

use chrono::Utc;
use planning_budget_core::{DebtId, TxId};
use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{
    DebtDto, DebtInput, DebtPatchDto, DebtPaymentStatusDto, DebtScheduleKindDto, DebtsOverviewDto,
    Int53, SchedulePaymentDto,
};
use crate::events::{ChangeScope, data_changed};
use crate::services::debts as service;
use crate::state::AppState;

/// Долг изменился: его график влияет и на план месяцев, поэтому уходят оба события.
fn changed(app: &AppHandle, dto: &DebtDto) {
    let months = service::touched_months(dto);
    data_changed(app, ChangeScope::Debts, months.clone());
    data_changed(app, ChangeScope::Plan, months);
}

#[tauri::command]
#[specta::specta]
pub async fn debts_list(
    state: State<'_, AppState>,
    month: String,
    include_closed: bool,
) -> Result<DebtsOverviewDto, AppError> {
    state
        .with_session(move |db| service::overview(db, &month, include_closed))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn debts_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: DebtInput,
) -> Result<DebtDto, AppError> {
    let dto = state
        .with_session(move |db| service::create(db, input, Utc::now()))
        .await?;
    changed(&app, &dto);
    Ok(dto)
}

/// Долг из траты со статусом «Долг»: сумма, статья и месяц берутся из траты.
#[tauri::command]
#[specta::specta]
pub async fn debt_from_tx(
    app: AppHandle,
    state: State<'_, AppState>,
    tx_id: Int53,
    lender: String,
    schedule: Vec<SchedulePaymentDto>,
) -> Result<DebtDto, AppError> {
    let dto = state
        .with_session(move |db| {
            service::create_from_transaction(db, TxId(tx_id.0), &lender, &schedule, Utc::now())
        })
        .await?;
    changed(&app, &dto);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn debts_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
    patch: DebtPatchDto,
) -> Result<DebtDto, AppError> {
    let dto = state
        .with_session(move |db| service::update(db, DebtId(id.0), patch, Utc::now()))
        .await?;
    changed(&app, &dto);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn debts_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<DebtDto, AppError> {
    let dto = state
        .with_session(move |db| service::delete(db, DebtId(id.0), Utc::now()))
        .await?;
    changed(&app, &dto);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn debts_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<DebtDto, AppError> {
    let dto = state
        .with_session(move |db| service::restore(db, DebtId(id.0), Utc::now()))
        .await?;
    changed(&app, &dto);
    Ok(dto)
}

/// Отметить строку графика оплаченной или вернуть её в план.
#[tauri::command]
#[specta::specta]
pub async fn debt_payment_set_status(
    app: AppHandle,
    state: State<'_, AppState>,
    payment_id: Int53,
    status: DebtPaymentStatusDto,
    paid_date: Option<String>,
) -> Result<DebtDto, AppError> {
    let dto = state
        .with_session(move |db| {
            service::set_payment_status(db, payment_id.0, status, paid_date.as_deref(), Utc::now())
        })
        .await?;
    changed(&app, &dto);
    Ok(dto)
}

/// Быстрый график погашения: считает Rust, данные не читаются, но сессия нужна.
#[tauri::command]
#[specta::specta]
pub async fn debt_schedule_preview(
    state: State<'_, AppState>,
    amount: Int53,
    taken_month: String,
    kind: DebtScheduleKindDto,
) -> Result<Vec<SchedulePaymentDto>, AppError> {
    state
        .with_session(move |_| service::schedule_preview(amount.0, &taken_month, kind))
        .await
}

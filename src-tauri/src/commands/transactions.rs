//! Траты и теги.

use chrono::Utc;
use planning_budget_core::TxId;
use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{
    Int53, TagDto, TransactionDto, TransactionInput, TransactionPatchDto, TxStatusDto,
    TxUpdateResultDto,
};
use crate::events::{ChangeScope, data_changed};
use crate::services::records as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn tx_list(
    state: State<'_, AppState>,
    month: String,
) -> Result<Vec<TransactionDto>, AppError> {
    state
        .with_session(move |db| service::tx_list(db, &month))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn tx_create(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: String,
    input: TransactionInput,
) -> Result<TransactionDto, AppError> {
    let dto = state
        .with_session_once(request_id, move |db| {
            service::tx_create(db, input, Utc::now())
        })
        .await?;
    data_changed(&app, ChangeScope::Transactions, vec![dto.month.clone()]);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn tx_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
    patch: TransactionPatchDto,
) -> Result<TxUpdateResultDto, AppError> {
    let dto = state
        .with_session(move |db| service::tx_update(db, TxId(id.0), patch, Utc::now()))
        .await?;
    let months = service::touched_months(&dto.previous.month, &dto.current.month);
    data_changed(&app, ChangeScope::Transactions, months);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn tx_set_status(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
    status: TxStatusDto,
) -> Result<TxUpdateResultDto, AppError> {
    let dto = state
        .with_session(move |db| service::tx_set_status(db, TxId(id.0), status, Utc::now()))
        .await?;
    data_changed(
        &app,
        ChangeScope::Transactions,
        vec![dto.current.month.clone()],
    );
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn tx_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<TransactionDto, AppError> {
    let dto = state
        .with_session(move |db| service::tx_delete(db, TxId(id.0), Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Transactions, vec![dto.month.clone()]);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn tx_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<TransactionDto, AppError> {
    let dto = state
        .with_session(move |db| service::tx_restore(db, TxId(id.0), Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Transactions, vec![dto.month.clone()]);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn tx_tags_set(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
    tag_ids: Vec<Int53>,
) -> Result<TransactionDto, AppError> {
    let dto = state
        .with_session(move |db| {
            service::tx_tags_set(db, TxId(id.0), tag_ids.into_iter().map(|t| t.0).collect())
        })
        .await?;
    data_changed(&app, ChangeScope::Transactions, vec![dto.month.clone()]);
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn tags_list(state: State<'_, AppState>) -> Result<Vec<TagDto>, AppError> {
    state.with_session(|db| service::tags_list(db)).await
}

#[tauri::command]
#[specta::specta]
pub async fn tags_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<TagDto, AppError> {
    let dto = state
        .with_session(move |db| service::tag_create(db, &name))
        .await?;
    data_changed(&app, ChangeScope::Tags, Vec::new());
    Ok(dto)
}

//! Категории, лимиты и план сбережений.

use budget_core::CategoryId;
use chrono::{Local, Utc};
use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{
    CategoryDto, CategoryInput, CategoryPatchDto, Int53, LimitEntryDto, SavingsRateDto,
};
use crate::events::{ChangeScope, data_changed};
use crate::services::categories as service;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn categories_list(
    state: State<'_, AppState>,
    include_archived: bool,
) -> Result<Vec<CategoryDto>, AppError> {
    state
        .with_session(move |db| service::list(db, include_archived))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn categories_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CategoryInput,
) -> Result<CategoryDto, AppError> {
    let dto = state
        .with_session(move |db| service::create(db, input, Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Categories, Vec::new());
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn categories_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
    patch: CategoryPatchDto,
) -> Result<CategoryDto, AppError> {
    let dto = state
        .with_session(move |db| service::update(db, CategoryId(id.0), patch, Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Categories, Vec::new());
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub async fn categories_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<(), AppError> {
    state
        .with_session(move |db| {
            service::archive(db, CategoryId(id.0), Local::now().date_naive(), Utc::now())
        })
        .await?;
    data_changed(&app, ChangeScope::Categories, Vec::new());
    // Архив категории сбережений обнуляет её процент: планы месяцев меняются.
    data_changed(&app, ChangeScope::Savings, Vec::new());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn categories_unarchive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<(), AppError> {
    state
        .with_session(move |db| service::unarchive(db, CategoryId(id.0), Utc::now()))
        .await?;
    data_changed(&app, ChangeScope::Categories, Vec::new());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn categories_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Int53,
) -> Result<(), AppError> {
    state
        .with_session(move |db| service::delete(db, CategoryId(id.0)))
        .await?;
    data_changed(&app, ChangeScope::Categories, Vec::new());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn categories_reorder(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<Int53>,
) -> Result<Vec<CategoryDto>, AppError> {
    let dtos = state
        .with_session(move |db| {
            service::reorder(db, ids.into_iter().map(|id| id.0).collect(), Utc::now())
        })
        .await?;
    data_changed(&app, ChangeScope::Categories, Vec::new());
    Ok(dtos)
}

#[tauri::command]
#[specta::specta]
pub async fn limits_set(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: Int53,
    valid_from: String,
    amount: Int53,
) -> Result<(), AppError> {
    state
        .with_session(move |db| {
            service::limits_set(db, CategoryId(category_id.0), &valid_from, amount.0)
        })
        .await?;
    data_changed(&app, ChangeScope::Limits, Vec::new());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn limits_clear(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: Int53,
    valid_from: String,
) -> Result<(), AppError> {
    state
        .with_session(move |db| service::limits_clear(db, CategoryId(category_id.0), &valid_from))
        .await?;
    data_changed(&app, ChangeScope::Limits, Vec::new());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn limits_history(
    state: State<'_, AppState>,
    category_id: Int53,
) -> Result<Vec<LimitEntryDto>, AppError> {
    state
        .with_session(move |db| service::limits_history(db, CategoryId(category_id.0)))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn savings_rates_list(
    state: State<'_, AppState>,
) -> Result<Vec<SavingsRateDto>, AppError> {
    state.with_session(|db| service::savings_rates(db)).await
}

#[tauri::command]
#[specta::specta]
pub async fn savings_rate_set(
    app: AppHandle,
    state: State<'_, AppState>,
    category_id: Int53,
    valid_from: String,
    rate_bp: i32,
) -> Result<(), AppError> {
    state
        .with_session(move |db| {
            service::savings_rate_set(db, CategoryId(category_id.0), &valid_from, rate_bp)
        })
        .await?;
    data_changed(&app, ChangeScope::Savings, Vec::new());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn savings_override_set(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
    category_id: Int53,
    rate_bp: i32,
) -> Result<(), AppError> {
    let month = state
        .with_session(move |db| {
            service::savings_override_set(db, &month, CategoryId(category_id.0), rate_bp)
        })
        .await?;
    data_changed(&app, ChangeScope::Savings, vec![month.to_string()]);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn savings_override_clear(
    app: AppHandle,
    state: State<'_, AppState>,
    month: String,
    category_id: Int53,
) -> Result<(), AppError> {
    let month = state
        .with_session(move |db| {
            service::savings_override_clear(db, &month, CategoryId(category_id.0))
        })
        .await?;
    data_changed(&app, ChangeScope::Savings, vec![month.to_string()]);
    Ok(())
}

//! Глобальный поиск палитры ⌘K.

use chrono::{Datelike as _, Local, Utc};
use tauri::State;

use crate::AppError;
use crate::dto::{
    CategoryUsageDto, FilterScreenDto, IncomeSearchDto, Int53, QueryParseDto, SavedFilterDto,
    SearchResultDto, TitleSuggestionDto, TransactionSearchDto,
};
use crate::services::search as service;
use crate::state::AppState;

fn current_year() -> u16 {
    u16::try_from(Local::now().year()).unwrap_or(2026)
}

#[tauri::command]
#[specta::specta]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    request_id: u32,
) -> Result<SearchResultDto, AppError> {
    let year = current_year();
    state
        .with_session(move |db| service::run(db, &query, request_id, year))
        .await
}

/// Разбор строки запроса для чипов (фильтр графика). Данных бюджета не читает.
#[tauri::command]
#[specta::specta]
pub async fn query_parse(query: String) -> Result<QueryParseDto, AppError> {
    Ok(service::query_parse(&query, current_year()))
}

#[tauri::command]
#[specta::specta]
pub async fn tx_search(
    state: State<'_, AppState>,
    query: String,
    request_id: u32,
) -> Result<TransactionSearchDto, AppError> {
    let year = current_year();
    state
        .with_session(move |db| service::tx_list(db, &query, request_id, year))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn incomes_search(
    state: State<'_, AppState>,
    query: String,
    request_id: u32,
) -> Result<IncomeSearchDto, AppError> {
    let year = current_year();
    state
        .with_session(move |db| service::income_list(db, &query, request_id, year))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn filters_list(state: State<'_, AppState>) -> Result<Vec<SavedFilterDto>, AppError> {
    state.with_session(|db| service::filters_list(db)).await
}

#[tauri::command]
#[specta::specta]
pub async fn filters_save(
    state: State<'_, AppState>,
    name: String,
    query: String,
    screen: FilterScreenDto,
) -> Result<SavedFilterDto, AppError> {
    state
        .with_session(move |db| service::filter_save(db, &name, &query, screen, Utc::now()))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn filters_delete(state: State<'_, AppState>, id: Int53) -> Result<(), AppError> {
    state
        .with_session(move |db| service::filter_delete(db, id.0))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn tx_suggest(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<TitleSuggestionDto>, AppError> {
    state
        .with_session(move |db| service::suggest(db, &query))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn tx_category_usage(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryUsageDto>, AppError> {
    let today = Local::now().date_naive();
    state
        .with_session(move |db| service::category_usage(db, today))
        .await
}

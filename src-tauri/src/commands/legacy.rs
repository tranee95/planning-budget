//! Перенос из старой таблицы xlsx. Файл выбирает пользователь в нативном диалоге на стороне
//! Rust: путь из WebView не принимается.

use chrono::{Local, Utc};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt as _;

use crate::AppError;
use crate::dto::LegacyReportDto;
use crate::events::{ChangeScope, data_changed};
use crate::services::legacy as service;
use crate::state::AppState;

/// Открывает диалог выбора `.xlsx`, переносит данные и возвращает отчёт сверки.
/// `null`, если пользователь закрыл диалог.
#[tauri::command]
#[specta::specta]
pub async fn legacy_import(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<LegacyReportDto>, AppError> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let path = handle
            .dialog()
            .file()
            .add_filter("Таблица Excel", &["xlsx"])
            .blocking_pick_file()
            .and_then(|p| p.into_path().ok())?;
        Some(service::read_file(&path))
    })
    .await
    .map_err(|e| AppError::internal("legacy dialog worker", &e))?;
    let Some(bytes) = picked.transpose()? else {
        return Ok(None);
    };
    let report = state
        .with_session(move |db| service::import(db, &bytes, Utc::now(), Local::now().date_naive()))
        .await?;
    data_changed(&app, ChangeScope::All, Vec::new());
    Ok(Some(report))
}

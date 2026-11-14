//! Резервные копии. Каталог и файл выбирает пользователь в
//! нативном диалоге на стороне Rust: путь из WebView не принимается.

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt as _;

use crate::AppError;
use crate::clock::Now;
use crate::dto::{BackupItemDto, BackupStatusDto, Secret};
use crate::events::{self, LockReason};
use crate::services::backup as service;
use crate::state::blocking;

#[tauri::command]
#[specta::specta]
pub async fn backup_status(app: AppHandle) -> Result<BackupStatusDto, AppError> {
    blocking(&app, |state| Ok(service::status(state))).await
}

/// Создаёт копию активного хранилища. Нужна открытая сессия (проверяет сервис).
#[tauri::command]
#[specta::specta]
pub async fn backup_create(app: AppHandle) -> Result<BackupItemDto, AppError> {
    blocking(&app, |state| service::create(state, Now::capture().utc)).await
}

/// Открывает диалог выбора каталога копий. `false`, если пользователь закрыл диалог.
#[tauri::command]
#[specta::specta]
pub async fn backup_pick_dir(app: AppHandle) -> Result<bool, AppError> {
    let handle = app.clone();
    blocking(&app, move |state| {
        let Some(chosen) = handle
            .dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
        else {
            return Ok(false);
        };
        service::set_dir(state, chosen).map(|()| true)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn backup_reset_dir(app: AppHandle) -> Result<(), AppError> {
    blocking(&app, service::reset_dir).await
}

/// Открывает копию из каталога как новое хранилище; пароль — от хранилища на момент копирования.
#[tauri::command]
#[specta::specta]
pub async fn backup_import_named(
    app: AppHandle,
    file_name: String,
    password: Secret,
    label: String,
) -> Result<(), AppError> {
    blocking(&app, move |state| {
        service::import_named(
            state,
            &file_name,
            password.secret(),
            &label,
            Now::capture().utc,
        )
    })
    .await?;
    // Активным стало другое хранилище: окно уходит на экран входа.
    events::notify_locked(&app, LockReason::Manual);
    Ok(())
}

/// То же для файла `.budgetbak`, выбранного в диалоге. `false`, если диалог закрыт.
#[tauri::command]
#[specta::specta]
pub async fn backup_import_pick(
    app: AppHandle,
    password: Secret,
    label: String,
) -> Result<bool, AppError> {
    let handle = app.clone();
    let imported = blocking(&app, move |state| {
        let Some(path) = handle
            .dialog()
            .file()
            .add_filter("Резервная копия", &["budgetbak"])
            .blocking_pick_file()
            .and_then(|p| p.into_path().ok())
        else {
            return Ok(false);
        };
        service::import(state, &path, password.secret(), &label, Now::capture().utc).map(|()| true)
    })
    .await?;
    if imported {
        events::notify_locked(&app, LockReason::Manual);
    }
    Ok(imported)
}

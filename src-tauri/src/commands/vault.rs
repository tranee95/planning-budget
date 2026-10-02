//! Команды сейфа. Работают до разблокировки, поэтому ходят не через `with_session`,
//! а через `state::blocking` (Argon2id и файлы не должны занимать async-рантайм).

use chrono::Utc;
use tauri::{AppHandle, Manager as _, State};
use tauri_plugin_dialog::DialogExt as _;

use crate::AppError;
use crate::dto::{RecoveryCodeDto, SavedDto, Secret, VaultStatusDto};
use crate::events::{self, LockReason};
use crate::services::vault as service;
use crate::state::{AppState, blocking};

#[tauri::command]
#[specta::specta]
pub async fn vault_status(app: AppHandle) -> Result<VaultStatusDto, AppError> {
    blocking(&app, |state| service::status(state, Utc::now())).await
}

#[tauri::command]
#[specta::specta]
pub async fn vault_create(app: AppHandle, password: Secret) -> Result<RecoveryCodeDto, AppError> {
    blocking(&app, move |state| {
        let code = service::create(state, password.secret(), Utc::now(), None)?;
        Ok(RecoveryCodeDto {
            recovery_code: code.to_string(),
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn vault_unlock(app: AppHandle, password: Secret) -> Result<(), AppError> {
    blocking(&app, move |state| {
        service::unlock(state, password.secret(), Utc::now())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn vault_unlock_recovery(
    app: AppHandle,
    code: Secret,
    new_password: Secret,
) -> Result<(), AppError> {
    blocking(&app, move |state| {
        service::unlock_recovery(state, code.secret(), new_password.secret(), Utc::now())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn vault_change_password(
    app: AppHandle,
    old_password: Secret,
    new_password: Secret,
) -> Result<(), AppError> {
    blocking(&app, move |state| {
        service::change_password(
            state,
            old_password.secret(),
            new_password.secret(),
            Utc::now(),
        )
    })
    .await
}

/// Перевыпуск ключа шифрования: нужен открытый сейф. Возвращает новый recovery-код.
#[tauri::command]
#[specta::specta]
pub async fn vault_rekey(app: AppHandle, password: Secret) -> Result<RecoveryCodeDto, AppError> {
    let result = blocking(&app, move |state| {
        let code = service::rekey(state, password.secret(), Utc::now())?;
        Ok(RecoveryCodeDto {
            recovery_code: code.to_string(),
        })
    })
    .await;
    // Сбой посреди перевыпуска закрывает сессию (см. `service::rekey`): окно должно об этом узнать.
    if result.is_err() && !app.state::<AppState>().is_unlocked() {
        events::notify_locked(&app, LockReason::Manual);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn vault_lock(app: AppHandle) -> Result<(), AppError> {
    events::lock_session(&app, LockReason::Manual);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn vault_reset(
    app: AppHandle,
    password: Secret,
    confirm_phrase: String,
) -> Result<(), AppError> {
    blocking(&app, move |state| {
        service::reset(state, password.secret(), &confirm_phrase, Utc::now())
    })
    .await?;
    // Сессию закрыл `reset`; окно узнаёт об этом тем же событием.
    events::notify_locked(&app, LockReason::Manual);
    Ok(())
}

/// Сохранение recovery-кода в файл. Диалог открывает Rust: путь из WebView не принимается.
#[tauri::command]
#[specta::specta]
pub async fn vault_save_recovery_code(app: AppHandle, code: Secret) -> Result<SavedDto, AppError> {
    let handle = app.clone();
    let saved = blocking(&app, move |state| {
        service::save_recovery_code(state, code.secret(), || {
            handle
                .dialog()
                .file()
                .set_file_name("budget-recovery-code.txt")
                .add_filter("Текст", &["txt"])
                .blocking_save_file()
                .and_then(|p| p.into_path().ok())
        })
    })
    .await?;
    Ok(SavedDto { saved })
}

/// Сброс таймера автоблокировки. Фронтенд шлёт на `pointerdown` и `keydown` не чаще раза в 30 с.
#[tauri::command]
#[specta::specta]
pub async fn activity_ping(state: State<'_, AppState>) -> Result<(), AppError> {
    state.idle().touch();
    Ok(())
}

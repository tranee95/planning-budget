//! Проверка и установка обновлений. Работают до разблокировки и данных не читают.

use tauri::AppHandle;
use tauri_specta::Event as _;

use crate::AppError;
use crate::dto::{UpdateState, UpdateStatusDto};
use crate::events::{LockReason, UpdateProgress, lock_session};
use crate::prefs;
use crate::services::updates as service;
use crate::state::AppState;

/// Проверяет обновления. `manual = false` — автопроверка при запуске: уважает тумблер в
/// «Настройках» и не чаще раза в сутки; `manual = true` — кнопка «Проверить сейчас».
#[tauri::command]
#[specta::specta]
pub async fn update_check(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    manual: bool,
) -> Result<UpdateStatusDto, AppError> {
    let path = state.paths().prefs();
    let current = prefs::load(&path);
    let now = service::now_secs();
    if !manual && !service::auto_check_due(current.check_updates, current.last_update_check, now) {
        return Ok(service::skipped(current.check_updates));
    }
    let status = service::check(&app).await;
    // Неудачная проверка не отодвигает следующую: при следующем запуске будет новая попытка.
    if status.state != UpdateState::Failed {
        prefs::set_last_update_check(&path, now)?;
    }
    Ok(status)
}

/// Скачивает обновление, проверяет подпись, закрывает сессию и ставит; затем приложение
/// перезапускается. Прогресс идёт событием `UpdateProgress`.
#[tauri::command]
#[specta::specta]
pub async fn update_install(app: AppHandle) -> Result<(), AppError> {
    let progress_app = app.clone();
    let lock_app = app.clone();
    service::install(
        &app,
        move |percent| {
            if let Err(err) = (UpdateProgress { percent }).emit(&progress_app) {
                tracing::warn!(%err, "UpdateProgress emit failed");
            }
        },
        move || lock_session(&lock_app, LockReason::Manual),
    )
    .await?;
    app.restart()
}

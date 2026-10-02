//! Настройки оболочки: работают до разблокировки, трогают только `ui-prefs.json`.

use tauri::{AppHandle, State};

use crate::AppError;
use crate::dto::{Prefs, PrefsPatch};
use crate::prefs;
use crate::state::AppState;
use crate::window;

#[tauri::command]
#[specta::specta]
pub async fn prefs_get(state: State<'_, AppState>) -> Result<Prefs, AppError> {
    Ok(prefs::load(&state.paths().prefs()))
}

/// Тёмная ли тема у окна. В режиме «системная» это тема ОС; изменения приходят событием
/// `SystemThemeChanged`.
#[tauri::command]
#[specta::specta]
pub async fn system_dark(app: AppHandle) -> Result<bool, AppError> {
    Ok(window::is_dark(&app))
}

#[tauri::command]
#[specta::specta]
pub async fn prefs_set(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: PrefsPatch,
) -> Result<Prefs, AppError> {
    let updated = prefs::update(&state.paths().prefs(), patch)?;
    // Заголовок и фон окна следуют теме сразу, а не после перезапуска.
    window::apply_theme(&app, &updated.theme);
    Ok(updated)
}

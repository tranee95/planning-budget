//! Типизированные события tauri-specta.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use crate::state::AppState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LockReason {
    Manual,
    Idle,
    /// Окно свёрнуто при включённой опции «блокировать при сворачивании».
    Minimize,
}

/// Хранилище заблокировано: фронтенд очищает stores с данными и уходит на `/lock`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct VaultLocked {
    pub reason: LockReason,
}

/// Тема ОС изменилась. WebView2 может не передавать её странице через `prefers-color-scheme`,
/// поэтому режим «системная» берёт тему у окна и следует этому событию.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SystemThemeChanged {
    pub dark: bool,
}

/// Закрывает сессию и сообщает окну. Без открытой сессии ничего не делает.
pub fn lock_session(app: &AppHandle, reason: LockReason) {
    if app.state::<AppState>().lock() {
        notify_locked(app, reason);
    }
}

/// Сообщает окну о блокировке, когда сессию уже закрыл сервис (`vault_reset`).
pub fn notify_locked(app: &AppHandle, reason: LockReason) {
    tracing::info!(?reason, "vault locked");
    if let Err(err) = (VaultLocked { reason }).emit(app) {
        tracing::warn!(%err, "VaultLocked emit failed");
    }
}

/// Что изменилось: по области фронтенд сбрасывает нужные кэши.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ChangeScope {
    Categories,
    Limits,
    Savings,
    Transactions,
    Incomes,
    Tags,
    Settings,
    /// Данные заменены целиком (dev-сид, будущий импорт xlsx): перечитать всё.
    All,
}

/// Данные бюджета изменились. `months` — затронутые месяцы `YYYY-MM`; пустой список
/// означает «не привязано к месяцу» (категории, теги, настройки) или «все месяцы».
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct DataChanged {
    pub scope: ChangeScope,
    pub months: Vec<String>,
}

/// Сообщает окну об изменении. Сбой отправки не отменяет уже сохранённую запись.
pub fn data_changed(app: &AppHandle, scope: ChangeScope, months: Vec<String>) {
    if let Err(err) = (DataChanged { scope, months }).emit(app) {
        tracing::warn!(%err, "DataChanged emit failed");
    }
}

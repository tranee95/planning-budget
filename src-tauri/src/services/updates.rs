//! Проверка и установка обновлений: единственное место, где приложение ходит в сеть.
//!
//! Запрос идёт к манифесту релиза публичного репозитория (`plugins.updater.endpoints`),
//! подпись установщика проверяет плагин по публичному ключу из конфигурации. В запрос не уходят
//! данные бюджета; в журнал пишутся только итог и код ошибки.

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt as _;

use crate::AppError;
use crate::dto::{UpdateState, UpdateStatusDto};

const DAY_SECS: i64 = 24 * 60 * 60;
const MAX_NOTES_CHARS: usize = 2_000;

/// Нужна ли автоматическая проверка: тумблер включён и последняя была не позже суток назад.
/// Часы, ушедшие назад (`last` в будущем), считаются просроченной проверкой.
#[must_use]
pub fn auto_check_due(enabled: bool, last_check: Option<i64>, now: i64) -> bool {
    enabled && last_check.is_none_or(|last| last > now || now - last >= DAY_SECS)
}

/// Текущее время в секундах Unix.
#[must_use]
pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn status(state: UpdateState) -> UpdateStatusDto {
    UpdateStatusDto {
        state,
        current_version: env!("CARGO_PKG_VERSION").to_owned(),
        version: None,
        notes: None,
    }
}

/// Статус без обращения к сети: автопроверка отключена или уже была.
#[must_use]
pub fn skipped(enabled: bool) -> UpdateStatusDto {
    status(if enabled {
        UpdateState::Skipped
    } else {
        UpdateState::Disabled
    })
}

/// Заметки релиза без управляющих символов и длиннее лимита: показываются как обычный текст.
fn clean_notes(body: Option<String>) -> Option<String> {
    let text: String = body?
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(MAX_NOTES_CHARS)
        .collect();
    let text = text.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Спрашивает манифест релиза. Сбой сети и неверная подпись — не ошибка приложения: итог `Failed`.
pub async fn check(app: &AppHandle) -> UpdateStatusDto {
    let result = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(err) => Err(err),
    };
    match result {
        Ok(Some(update)) => {
            tracing::info!(available = true, "update check");
            UpdateStatusDto {
                version: Some(update.version.clone()),
                notes: clean_notes(update.body.clone()),
                ..status(UpdateState::Available)
            }
        }
        Ok(None) => {
            tracing::info!(available = false, "update check");
            status(UpdateState::UpToDate)
        }
        Err(err) => {
            // Текст ошибки плагина может содержать адрес манифеста: в журнал идёт только вид.
            tracing::warn!(kind = error_kind(&err), "update check failed");
            status(UpdateState::Failed)
        }
    }
}

/// Скачивает и ставит обновление, проверив подпись. `progress` получает процент 0–100 (или
/// `None`, если размер неизвестен). `before_install` вызывается после загрузки и проверки,
/// непосредственно перед установкой; после успеха вызывающий перезапускает приложение.
///
/// # Errors
/// `Io`, если обновление не найдено, не скачалось или подпись не сошлась.
pub async fn install(
    app: &AppHandle,
    progress: impl Fn(Option<u8>),
    before_install: impl FnOnce(),
) -> Result<(), AppError> {
    let fail = |kind: &'static str| {
        tracing::warn!(kind, "update install failed");
        AppError::Io {
            message_key: "errors.io.update".into(),
        }
    };
    let updater = app.updater().map_err(|e| fail(error_kind(&e)))?;
    let update = updater
        .check()
        .await
        .map_err(|e| fail(error_kind(&e)))?
        .ok_or_else(|| fail("no_update"))?;
    let mut downloaded: u64 = 0;
    let bytes = update
        .download(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                progress(total.filter(|t| *t > 0).map(|t| percent(downloaded, t)));
            },
            || {},
        )
        .await
        .map_err(|e| fail(error_kind(&e)))?;
    // Установщик заменяет файлы и закрывает приложение: сессия закрывается до него.
    before_install();
    update.install(bytes).map_err(|e| fail(error_kind(&e)))
}

/// Доля скачанного в процентах, не больше 100.
fn percent(done: u64, total: u64) -> u8 {
    u8::try_from((u128::from(done) * 100 / u128::from(total.max(1))).min(100)).unwrap_or(100)
}

/// Вид ошибки плагина для журнала, без текста (в нём может быть адрес).
fn error_kind(err: &tauri_plugin_updater::Error) -> &'static str {
    use tauri_plugin_updater::Error as E;
    match err {
        E::Network(_) | E::Reqwest(_) => "network",
        E::Minisign(_) | E::SignatureUtf8(_) | E::Base64(_) => "signature",
        E::EmptyEndpoints | E::ReleaseNotFound | E::UnsupportedArch | E::UnsupportedOs => "config",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn auto_check_runs_when_enabled_and_never_checked() {
        assert!(auto_check_due(true, None, NOW));
    }

    #[test]
    fn auto_check_is_off_when_the_switch_is_off() {
        assert!(!auto_check_due(false, None, NOW));
        assert!(!auto_check_due(false, Some(NOW - 10 * DAY_SECS), NOW));
    }

    #[test]
    fn auto_check_waits_a_day_between_checks() {
        assert!(!auto_check_due(true, Some(NOW - DAY_SECS + 1), NOW));
        assert!(auto_check_due(true, Some(NOW - DAY_SECS), NOW));
        assert!(auto_check_due(true, Some(NOW - 3 * DAY_SECS), NOW));
    }

    #[test]
    fn auto_check_treats_a_future_stamp_as_overdue() {
        assert!(auto_check_due(true, Some(NOW + 3_600), NOW));
    }

    #[test]
    fn skipped_distinguishes_disabled_from_recent() {
        assert_eq!(skipped(false).state, UpdateState::Disabled);
        assert_eq!(skipped(true).state, UpdateState::Skipped);
        assert_eq!(skipped(true).current_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn notes_are_plain_text_and_capped() {
        assert_eq!(clean_notes(None), None);
        assert_eq!(clean_notes(Some("  \n ".into())), None);
        assert_eq!(
            clean_notes(Some("Исправления\u{7}\nи улучшения".into())).as_deref(),
            Some("Исправления\nи улучшения")
        );
        let long = "я".repeat(MAX_NOTES_CHARS + 500);
        assert_eq!(
            clean_notes(Some(long)).map(|t| t.chars().count()),
            Some(MAX_NOTES_CHARS)
        );
    }

    #[test]
    fn percent_is_bounded() {
        assert_eq!(percent(0, 200), 0);
        assert_eq!(percent(50, 200), 25);
        assert_eq!(percent(200, 200), 100);
        assert_eq!(percent(900, 200), 100);
        assert_eq!(percent(5, 0), 100);
    }
}

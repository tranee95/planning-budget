//! `ui-prefs.json`: тема, язык, масштаб. Читается до разблокировки.

use std::fs;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use crate::AppError;
use crate::dto::{Prefs, PrefsPatch};

const SCALE_RANGE: std::ops::RangeInclusive<u8> = 90..=125;
const MAX_LOCALE_LEN: usize = 16;

/// Файл настроек с запасом на будущее: ключ, которого нет в старом файле, не сбрасывает остальные
/// значения (в `Prefs` поля обязательны, чтобы TS-типы не стали необязательными).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnDisk {
    #[serde(default = "default_theme")]
    theme: crate::dto::Theme,
    #[serde(default = "default_locale")]
    locale: String,
    #[serde(default = "default_scale")]
    ui_scale: u8,
    #[serde(default = "default_motion")]
    reduced_motion: crate::dto::ReducedMotion,
    #[serde(default)]
    autolock_minutes: Option<u16>,
    /// Нет ключа в существующем файле — установка старше подсказок: её владелец не новичок.
    #[serde(default)]
    show_tips: Option<bool>,
}

fn default_theme() -> crate::dto::Theme {
    Prefs::default().theme
}

fn default_motion() -> crate::dto::ReducedMotion {
    Prefs::default().reduced_motion
}

fn default_locale() -> String {
    Prefs::default().locale
}

fn default_scale() -> u8 {
    Prefs::default().ui_scale
}

/// Чтение-изменение-запись файла идут по одному в процессе: иначе две команды подряд
/// (смена темы и сохранение настройки блокировки) затирали бы изменения друг друга.
static WRITE: Mutex<()> = Mutex::new(());

/// Читает настройки. Нет файла или он повреждён — значения по умолчанию:
/// из-за косметических настроек приложение не должно отказываться стартовать.
#[must_use]
pub fn load(path: &Path) -> Prefs {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<OnDisk>(&bytes).ok())
        .map(|d| Prefs {
            theme: d.theme,
            locale: d.locale,
            ui_scale: d.ui_scale,
            reduced_motion: d.reduced_motion,
            autolock_minutes: d.autolock_minutes,
            show_tips: d.show_tips.unwrap_or(false),
        })
        .unwrap_or_default()
}

/// Запись через соседний файл и переименование: сбой посреди записи не оставляет обрезанный
/// JSON, после которого `load` сбросил бы тему, масштаб и подсказки. В файле нет данных бюджета.
fn save(path: &Path, prefs: &Prefs) -> Result<(), AppError> {
    let io = || AppError::Io {
        message_key: "errors.io.prefs".into(),
    };
    let bytes = serde_json::to_vec_pretty(prefs).map_err(|e| AppError::internal("prefs", &e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(|_| io())?;
    fs::rename(&tmp, path).map_err(|_| io())
}

/// Применяет частичное обновление и сохраняет результат.
///
/// # Errors
/// `Validation` для недопустимых значений, `Io` при ошибке записи.
pub fn update(path: &Path, patch: PrefsPatch) -> Result<Prefs, AppError> {
    let _guard = WRITE.lock().unwrap_or_else(PoisonError::into_inner);
    let mut prefs = load(path);
    if let Some(theme) = patch.theme {
        prefs.theme = theme;
    }
    if let Some(motion) = patch.reduced_motion {
        prefs.reduced_motion = motion;
    }
    if let Some(tips) = patch.show_tips {
        prefs.show_tips = tips;
    }
    if let Some(scale) = patch.ui_scale {
        if !SCALE_RANGE.contains(&scale) {
            return Err(invalid("uiScale"));
        }
        prefs.ui_scale = scale;
    }
    if let Some(locale) = patch.locale {
        let valid = !locale.is_empty()
            && locale.len() <= MAX_LOCALE_LEN
            && locale
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valid {
            return Err(invalid("locale"));
        }
        prefs.locale = locale;
    }
    save(path, &prefs)?;
    Ok(prefs)
}

/// Записывает копию порога автоблокировки для экрана входа (`None` — после сброса данных: порога
/// больше нет); файл не трогается, если значение то же.
///
/// # Errors
/// `Io` при ошибке записи.
pub fn set_autolock_minutes(path: &Path, minutes: Option<u16>) -> Result<(), AppError> {
    let _guard = WRITE.lock().unwrap_or_else(PoisonError::into_inner);
    let mut prefs = load(path);
    if prefs.autolock_minutes == minutes {
        return Ok(());
    }
    prefs.autolock_minutes = minutes;
    save(path, &prefs)
}

fn invalid(field: &str) -> AppError {
    AppError::Validation {
        message_key: "errors.prefs.invalid".into(),
        field: Some(field.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{ReducedMotion, Theme};

    #[test]
    fn old_file_without_autolock_keeps_the_other_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        fs::write(
            &path,
            br#"{"theme":"dark","locale":"ru-RU","uiScale":110,"reducedMotion":"system"}"#,
        )
        .unwrap();
        let prefs = load(&path);
        assert_eq!(prefs.theme, Theme::Dark);
        assert_eq!(prefs.ui_scale, 110);
        // Порог неизвестен, пока бэкенд его не запишет: экран входа не должен называть выдуманное число.
        assert_eq!(prefs.autolock_minutes, None);
        // Существующий файл без ключа: установка старше подсказок, показывать их не нужно.
        assert!(!prefs.show_tips);
    }

    #[test]
    fn fresh_install_shows_tips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        assert!(load(&path).show_tips);
        let created = update(&path, PrefsPatch::default()).unwrap();
        assert!(created.show_tips);
        assert!(load(&path).show_tips);
    }

    #[test]
    fn concurrent_updates_do_not_lose_each_other_and_leave_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for _ in 0..30 {
                    set_autolock_minutes(&path, Some(15)).unwrap();
                }
            });
            scope.spawn(|| {
                for _ in 0..30 {
                    let patch = PrefsPatch {
                        theme: Some(Theme::Dark),
                        ..PrefsPatch::default()
                    };
                    update(&path, patch).unwrap();
                }
            });
        });
        let prefs = load(&path);
        assert_eq!(
            (prefs.autolock_minutes, prefs.theme),
            (Some(15), Theme::Dark)
        );
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn tips_switch_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        let patch = PrefsPatch {
            show_tips: Some(false),
            ..PrefsPatch::default()
        };
        assert!(!update(&path, patch).unwrap().show_tips);
        assert!(!load(&path).show_tips);
    }

    #[test]
    fn autolock_mirror_is_written_once_and_keeps_other_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        update(
            &path,
            PrefsPatch {
                theme: Some(Theme::Dark),
                ..PrefsPatch::default()
            },
        )
        .unwrap();
        set_autolock_minutes(&path, Some(15)).unwrap();
        let prefs = load(&path);
        assert_eq!(
            (prefs.autolock_minutes, prefs.theme),
            (Some(15), Theme::Dark)
        );
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        set_autolock_minutes(&path, Some(15)).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn missing_or_corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        assert_eq!(load(&path), Prefs::default());
        fs::write(&path, b"{ not json").unwrap();
        assert_eq!(load(&path), Prefs::default());
    }

    #[test]
    fn patch_changes_only_given_fields_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        let after = update(
            &path,
            PrefsPatch {
                theme: Some(Theme::Dark),
                ui_scale: Some(110),
                ..PrefsPatch::default()
            },
        )
        .unwrap();
        assert_eq!(after.theme, Theme::Dark);
        assert_eq!(after.ui_scale, 110);
        assert_eq!(after.locale, "ru-RU");
        assert_eq!(after.reduced_motion, ReducedMotion::System);
        assert_eq!(load(&path), after);
    }

    #[test]
    fn invalid_values_are_rejected_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        for patch in [
            PrefsPatch {
                ui_scale: Some(89),
                ..PrefsPatch::default()
            },
            PrefsPatch {
                ui_scale: Some(126),
                ..PrefsPatch::default()
            },
            PrefsPatch {
                locale: Some("ru RU/../x".into()),
                ..PrefsPatch::default()
            },
        ] {
            assert!(matches!(
                update(&path, patch),
                Err(AppError::Validation { .. })
            ));
        }
        assert!(!path.exists());
    }

    #[test]
    fn file_holds_only_shell_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui-prefs.json");
        update(&path, PrefsPatch::default()).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "autolockMinutes",
                "locale",
                "reducedMotion",
                "showTips",
                "theme",
                "uiScale"
            ]
        );
    }
}

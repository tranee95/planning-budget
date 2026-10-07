//! Настройки интерфейса (`ui-prefs.json`), работают до разблокировки.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ReducedMotion {
    System,
    On,
}

/// Содержимое `ui-prefs.json`: только оболочка, никаких данных бюджета.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub theme: Theme,
    pub locale: String,
    /// Масштаб интерфейса в процентах, 90–125.
    pub ui_scale: u8,
    pub reduced_motion: ReducedMotion,
    /// Копия `security.autolock_minutes` для экрана входа: настройка лежит в зашифрованной БД и
    /// до входа недоступна. Пишет бэкенд при входе и при смене настройки; не секрет. `0` — никогда,
    /// `None` — ещё не известна (файл создан старой версией): экран входа тогда ничего не пишет.
    pub autolock_minutes: Option<u16>,
    /// Знакомство при первом запуске пройдено или пропущено.
    pub intro_done: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            locale: "ru-RU".to_owned(),
            ui_scale: 100,
            reduced_motion: ReducedMotion::System,
            autolock_minutes: None,
            intro_done: false,
        }
    }
}

#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PrefsPatch {
    pub theme: Option<Theme>,
    pub locale: Option<String>,
    pub ui_scale: Option<u8>,
    pub reduced_motion: Option<ReducedMotion>,
    pub intro_done: Option<bool>,
}

//! Настройки расчётов (хранятся в БД).

use serde::{Deserialize, Serialize};
use specta::Type;

/// Настройки, которые редактирует пользователь.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub savings_min_bp: i32,
    pub savings_norm_bp: i32,
    pub savings_max_bp: i32,
    pub weeks_per_month: u8,
    pub autolock_minutes: u16,
    pub lock_on_minimize: bool,
    /// Автокопии включены (ключ `backup.auto_daily`: имя осталось с тех пор, когда расписание было одно).
    pub backup_auto: bool,
    /// Не чаще чем раз в столько календарных дней, 1–90.
    pub backup_interval_days: u8,
}

#[derive(Debug, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatchDto {
    pub savings_min_bp: Option<i32>,
    pub savings_norm_bp: Option<i32>,
    pub savings_max_bp: Option<i32>,
    pub weeks_per_month: Option<u8>,
    pub autolock_minutes: Option<u16>,
    pub lock_on_minimize: Option<bool>,
    pub backup_auto: Option<bool>,
    pub backup_interval_days: Option<u8>,
}

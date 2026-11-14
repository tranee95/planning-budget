//! Настройки пользователя.

use std::collections::BTreeMap;

use planning_budget_storage::Db;
use serde_json::Value;

use crate::AppError;
use crate::dto::{SettingsDto, SettingsPatchDto};
use crate::prefs;
use crate::state::AppState;

fn missing() -> AppError {
    AppError::internal("settings", &"required setting is missing or malformed")
}

fn int(map: &BTreeMap<String, Value>, key: &str) -> Result<i64, AppError> {
    map.get(key).and_then(Value::as_i64).ok_or_else(missing)
}

fn flag(map: &BTreeMap<String, Value>, key: &str) -> Result<bool, AppError> {
    map.get(key).and_then(Value::as_bool).ok_or_else(missing)
}

/// Базы, созданные до появления расписания, ключа не содержат: для них копия раз в день.
const DEFAULT_BACKUP_INTERVAL_DAYS: u8 = 1;

fn interval_days(map: &BTreeMap<String, Value>) -> u8 {
    map.get("backup.interval_days")
        .and_then(Value::as_i64)
        .and_then(|v| u8::try_from(v).ok())
        .unwrap_or(DEFAULT_BACKUP_INTERVAL_DAYS)
}

/// Расписание автоматических копий: включены ли и как часто (в днях).
pub fn backup_schedule(db: &Db) -> Result<(bool, u8), AppError> {
    let map = db.settings_map()?;
    Ok((flag(&map, "backup.auto_daily")?, interval_days(&map)))
}

pub fn get(db: &Db) -> Result<SettingsDto, AppError> {
    let map = db.settings_map()?;
    let to_i32 = |key| i32::try_from(int(&map, key)?).map_err(|_| missing());
    Ok(SettingsDto {
        savings_min_bp: to_i32("savings.target_min_bp")?,
        savings_norm_bp: to_i32("savings.target_norm_bp")?,
        savings_max_bp: to_i32("savings.target_max_bp")?,
        weeks_per_month: u8::try_from(int(&map, "ui.weeks_per_month")?).map_err(|_| missing())?,
        autolock_minutes: u16::try_from(int(&map, "security.autolock_minutes")?)
            .map_err(|_| missing())?,
        lock_on_minimize: flag(&map, "security.lock_on_minimize")?,
        backup_auto: flag(&map, "backup.auto_daily")?,
        backup_interval_days: interval_days(&map),
    })
}

/// Применяет настройки безопасности к работающему приложению: порог автоблокировки, блокировку
/// при сворачивании и копию порога для экрана входа. Запись копии необязательна для работы:
/// ошибка только попадает в лог.
pub fn apply_security(state: &AppState, autolock_minutes: u16, lock_on_minimize: bool) {
    state.idle().set_minutes(u32::from(autolock_minutes));
    state.set_lock_on_minimize(lock_on_minimize);
    if let Err(err) = prefs::set_autolock_minutes(&state.paths().prefs(), Some(autolock_minutes)) {
        tracing::warn!(?err, "autolock mirror not written");
    }
}

/// Записывает только переданные поля и возвращает настройки целиком.
pub fn set(db: &mut Db, patch: SettingsPatchDto) -> Result<SettingsDto, AppError> {
    let changes: [(&str, Option<Value>); 8] = [
        (
            "savings.target_min_bp",
            patch.savings_min_bp.map(Value::from),
        ),
        (
            "savings.target_norm_bp",
            patch.savings_norm_bp.map(Value::from),
        ),
        (
            "savings.target_max_bp",
            patch.savings_max_bp.map(Value::from),
        ),
        ("ui.weeks_per_month", patch.weeks_per_month.map(Value::from)),
        (
            "security.autolock_minutes",
            patch.autolock_minutes.map(Value::from),
        ),
        (
            "security.lock_on_minimize",
            patch.lock_on_minimize.map(Value::from),
        ),
        ("backup.auto_daily", patch.backup_auto.map(Value::from)),
        (
            "backup.interval_days",
            patch.backup_interval_days.map(Value::from),
        ),
    ];
    let given: Vec<(&str, Value)> = changes
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect();
    db.settings_set_many(&given)?;
    get(db)
}

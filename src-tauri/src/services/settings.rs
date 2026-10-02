//! Настройки пользователя.

use std::collections::BTreeMap;

use budget_storage::Db;
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

pub fn get(db: &Db) -> Result<SettingsDto, AppError> {
    let map = db.settings_map()?;
    let to_i32 = |key| i32::try_from(int(&map, key)?).map_err(|_| missing());
    Ok(SettingsDto {
        savings_min_bp: to_i32("savings.target_min_bp")?,
        savings_norm_bp: to_i32("savings.target_norm_bp")?,
        savings_max_bp: to_i32("savings.target_max_bp")?,
        bonds_rate_bp: to_i32("bonds.rate_bp")?,
        bonds_coupon_tax_bp: to_i32("bonds.coupon_tax_bp")?,
        bonds_initial_balance: int(&map, "bonds.initial_balance")?,
        bonds_initial_month: map
            .get("bonds.initial_month")
            .and_then(Value::as_str)
            .ok_or_else(missing)?
            .to_owned(),
        weeks_per_month: u8::try_from(int(&map, "ui.weeks_per_month")?).map_err(|_| missing())?,
        autolock_minutes: u16::try_from(int(&map, "security.autolock_minutes")?)
            .map_err(|_| missing())?,
        lock_on_minimize: flag(&map, "security.lock_on_minimize")?,
        backup_auto_daily: flag(&map, "backup.auto_daily")?,
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
    let changes: [(&str, Option<Value>); 11] = [
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
        ("bonds.rate_bp", patch.bonds_rate_bp.map(Value::from)),
        (
            "bonds.coupon_tax_bp",
            patch.bonds_coupon_tax_bp.map(Value::from),
        ),
        (
            "bonds.initial_balance",
            patch.bonds_initial_balance.map(Value::from),
        ),
        (
            "bonds.initial_month",
            patch.bonds_initial_month.map(Value::from),
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
        (
            "backup.auto_daily",
            patch.backup_auto_daily.map(Value::from),
        ),
    ];
    let given: Vec<(&str, Value)> = changes
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect();
    db.settings_set_many(&given)?;
    get(db)
}

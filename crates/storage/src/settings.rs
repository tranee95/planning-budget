//! Настройки: ключ → JSON-значение.

use std::collections::BTreeMap;

use planning_budget_core::{BasisPoints, Settings};
use rusqlite::params;
use serde_json::Value;

use crate::{Db, StorageError};

/// Допустимый вид значения ключа.
#[derive(Clone, Copy)]
enum Shape {
    BasisPoints,
    Int { min: i64, max: i64 },
    Bool,
    Text,
}

/// Все известные ключи: незнакомый ключ записать нельзя.
const KEYS: [(&str, Shape); 9] = [
    ("savings.target_min_bp", Shape::BasisPoints),
    ("savings.target_norm_bp", Shape::BasisPoints),
    ("savings.target_max_bp", Shape::BasisPoints),
    ("ui.weeks_per_month", Shape::Int { min: 1, max: 5 }),
    (
        "security.autolock_minutes",
        Shape::Int { min: 0, max: 1440 },
    ),
    ("security.lock_on_minimize", Shape::Bool),
    ("backup.auto_daily", Shape::Bool),
    ("currency", Shape::Text),
    ("locale", Shape::Text),
];

fn validate(shape: Shape, value: &Value) -> bool {
    match shape {
        Shape::BasisPoints => value.as_i64().is_some_and(|v| (0..=10_000).contains(&v)),
        Shape::Int { min, max } => value.as_i64().is_some_and(|v| (min..=max).contains(&v)),
        Shape::Bool => value.is_boolean(),
        Shape::Text => value.as_str().is_some_and(|s| !s.trim().is_empty()),
    }
}

fn int(map: &BTreeMap<String, Value>, key: &str) -> Result<i64, StorageError> {
    map.get(key)
        .and_then(Value::as_i64)
        .ok_or(StorageError::Corrupt)
}

fn basis_points(map: &BTreeMap<String, Value>, key: &str) -> Result<BasisPoints, StorageError> {
    i32::try_from(int(map, key)?)
        .map(BasisPoints)
        .map_err(|_| StorageError::Corrupt)
}

impl Db {
    /// Все настройки как JSON-значения.
    ///
    /// # Errors
    /// Ошибка SQLite или значение, не разбирающееся как JSON.
    pub fn settings_map(&self) -> Result<BTreeMap<String, Value>, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT key, value FROM settings")?;
        let mut rows = stmt.query([])?;
        let mut out = BTreeMap::new();
        while let Some(row) = rows.next()? {
            let value: String = row.get(1)?;
            out.insert(
                row.get(0)?,
                serde_json::from_str(&value).map_err(|_| StorageError::Corrupt)?,
            );
        }
        Ok(out)
    }

    /// Записывает одну настройку.
    ///
    /// # Errors
    /// Те же, что у [`Db::settings_set_many`].
    pub fn setting_set(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
        self.settings_set_many(&[(key, value.clone())])
    }

    /// Записывает набор настроек одной транзакцией: при любой ошибке не меняется ни одна.
    ///
    /// # Errors
    /// `Invalid("settings.unknown_key")` для незнакомого ключа, `Invalid("settings.bad_value")`
    /// для значения не того вида или вне диапазона, `Invalid("settings.corridor_order")`, если
    /// после записи нарушено `savings.target_min_bp ≤ target_norm_bp ≤ target_max_bp`.
    pub fn settings_set_many(&mut self, changes: &[(&str, Value)]) -> Result<(), StorageError> {
        for (key, value) in changes {
            let Some((_, shape)) = KEYS.iter().find(|(name, _)| name == key) else {
                return Err(StorageError::Invalid("settings.unknown_key"));
            };
            if !validate(*shape, value) {
                return Err(StorageError::Invalid("settings.bad_value"));
            }
        }
        let tx = self.conn.transaction()?;
        {
            let mut upsert = tx.prepare_cached(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            )?;
            for (key, value) in changes {
                upsert.execute(params![key, value.to_string()])?;
            }
        }
        let corridor: (i64, i64, i64) = tx.query_row(
            "SELECT
                 (SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'savings.target_min_bp'),
                 (SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'savings.target_norm_bp'),
                 (SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'savings.target_max_bp')",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if !(corridor.0 <= corridor.1 && corridor.1 <= corridor.2) {
            return Err(StorageError::Invalid("settings.corridor_order"));
        }
        tx.commit()?;
        Ok(())
    }

    /// Настройки, влияющие на расчёты.
    ///
    /// # Errors
    /// [`StorageError::Corrupt`], если нет обязательного ключа или значение испорчено.
    pub fn settings(&self) -> Result<Settings, StorageError> {
        let map = self.settings_map()?;
        Ok(Settings {
            savings_min: basis_points(&map, "savings.target_min_bp")?,
            savings_norm: basis_points(&map, "savings.target_norm_bp")?,
            savings_max: basis_points(&map, "savings.target_max_bp")?,
            weeks_per_month: u8::try_from(int(&map, "ui.weeks_per_month")?)
                .map_err(|_| StorageError::Corrupt)?,
        })
    }
}

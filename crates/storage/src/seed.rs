//! Сиды при создании хранилища: категории, лимиты, настройки.

use chrono::{DateTime, SecondsFormat, Utc};
use planning_budget_core::analytics::standard_dashboard;
use rusqlite::params;
use serde::Deserialize;

use crate::writes::{
    NewCategory, insert_category, insert_limit, insert_rate, upsert_savings_params,
};
use crate::{Db, StorageError};

/// Палитра категорий. Цвета сида назначаются по кругу
/// в порядке `sort_order`: цвета таблицы-источника в палитру не входят.
pub(crate) const PALETTE: [&str; 10] = [
    "#3AA567", "#5B7FD6", "#8C6BD1", "#D9893A", "#DD5249", "#C552A0", "#4E8F6E", "#E0A73A",
    "#6C7480", "#3445D0",
];

/// Процент плана у сидовой категории сбережений: равен `savings.target_norm_bp`.
const DEFAULT_SAVINGS_RATE_BP: i64 = 1400;

/// Ставка и налог накопления сидовой категории сбережений.
const DEFAULT_ANNUAL_RATE_BP: i64 = 1600;
const DEFAULT_COUPON_TAX_BP: i64 = 1300;

const STANDARD_DASHBOARD_NAME: &str = "Мой бюджет";

const CATEGORIES_JSON: &str = include_str!("../assets/seed_categories.json");

/// Значения по умолчанию (JSON-значения).
const DEFAULT_SETTINGS: [(&str, &str); 9] = [
    ("savings.target_min_bp", "1300"),
    ("savings.target_norm_bp", "1400"),
    ("savings.target_max_bp", "1500"),
    ("ui.weeks_per_month", "4"),
    ("security.autolock_minutes", "5"),
    ("security.lock_on_minimize", "false"),
    ("backup.auto_daily", "true"),
    ("currency", "\"RUB\""),
    ("locale", "\"ru-RU\""),
];

#[derive(Deserialize)]
struct SeedCategory {
    name: String,
    kind: String,
    /// Копейки; `None` у категории сбережений: её лимит равен плану сбережений.
    limit: Option<i64>,
    sort_order: i64,
    note: Option<String>,
}

impl Db {
    /// Заливает категории, лимиты и процент сбережений (`valid_from` = месяц `now`), стандартный
    /// дашборд и настройки по умолчанию.
    ///
    /// Повторный вызов ничего не меняет: категории и дашборд сидируются только в пустую
    /// таблицу, настройки — только отсутствующие ключи.
    ///
    /// # Errors
    /// Ошибка SQLite или битый встроенный ресурс (последнее ловит тест).
    pub fn seed_defaults(&mut self, now: DateTime<Utc>) -> Result<(), StorageError> {
        let categories: Vec<SeedCategory> =
            serde_json::from_str(CATEGORIES_JSON).map_err(|_| StorageError::SeedAsset)?;
        let stamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);
        let month = now.format("%Y-%m").to_string();

        let tx = self.conn.transaction()?;
        let existing: i64 = tx.query_row("SELECT count(*) FROM categories", [], |r| r.get(0))?;
        if existing == 0 {
            for (idx, cat) in categories.iter().enumerate() {
                let color = PALETTE
                    .get(idx % PALETTE.len())
                    .copied()
                    .unwrap_or("#6C7480");
                let id = insert_category(
                    &tx,
                    &NewCategory {
                        id: None,
                        name: &cat.name,
                        kind: &cat.kind,
                        color,
                        sort_order: cat.sort_order,
                        note: cat.note.as_deref(),
                        archived_at: None,
                        stamp: &stamp,
                    },
                )?;
                if let Some(amount) = cat.limit {
                    insert_limit(&tx, id, &month, amount)?;
                }
                if cat.kind == "savings" {
                    insert_rate(&tx, id, &month, DEFAULT_SAVINGS_RATE_BP, None)?;
                    upsert_savings_params(
                        &tx,
                        id,
                        DEFAULT_ANNUAL_RATE_BP,
                        DEFAULT_COUPON_TAX_BP,
                        0,
                        &month,
                    )?;
                }
            }
        }
        insert_standard_dashboard(&tx, &stamp)?;
        {
            let mut insert_setting =
                tx.prepare_cached("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")?;
            for (key, value) in DEFAULT_SETTINGS {
                insert_setting.execute(params![key, value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Добавляет стандартный дашборд по явному действию пользователя. Нужен хранилищам,
    /// созданным до, и тем, где дашбордов не осталось: сам сид его не возвращает.
    ///
    /// # Errors
    /// `Conflict`, если дашборд уже есть; ошибка SQLite.
    pub fn seed_standard_dashboard(&mut self, now: DateTime<Utc>) -> Result<(), StorageError> {
        let stamp = now.to_rfc3339_opts(SecondsFormat::Secs, true);
        let tx = self.conn.transaction()?;
        if !insert_standard_dashboard(&tx, &stamp)? {
            return Err(StorageError::Conflict("dashboard.exists"));
        }
        tx.commit()?;
        Ok(())
    }
}

/// Вставляет стандартный дашборд, если таблица `dashboards` пуста; иначе ничего не меняет.
/// Возвращает `true`, если дашборд добавлен.
fn insert_standard_dashboard(
    tx: &rusqlite::Transaction<'_>,
    stamp: &str,
) -> Result<bool, StorageError> {
    let dashboards: i64 = tx.query_row("SELECT count(*) FROM dashboards", [], |r| r.get(0))?;
    if dashboards != 0 {
        return Ok(false);
    }
    tx.execute(
        "INSERT INTO dashboards (name, sort_order, is_default) VALUES (?1, 0, 1)",
        [STANDARD_DASHBOARD_NAME],
    )?;
    let dashboard_id = tx.last_insert_rowid();
    let mut insert_chart = tx.prepare_cached(
        "INSERT INTO charts (dashboard_id, spec, x, y, w, h, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
    )?;
    for chart in standard_dashboard() {
        let spec = serde_json::to_string(&chart.spec).map_err(|_| StorageError::SeedAsset)?;
        insert_chart.execute(params![
            dashboard_id,
            spec,
            chart.x,
            chart.y,
            chart.w,
            chart.h,
            stamp
        ])?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_categories_parse_and_have_sequential_order() {
        let cats: Vec<SeedCategory> = serde_json::from_str(CATEGORIES_JSON).unwrap();
        assert_eq!(cats.len(), 17);
        // Первый старт обобщённый: без личных названий, лимитов и пометок (их задаёт пользователь).
        assert!(cats.iter().all(|c| c.limit.is_none() && c.note.is_none()));
        for (idx, cat) in cats.iter().enumerate() {
            assert_eq!(cat.sort_order, i64::try_from(idx).unwrap());
        }
    }

    #[test]
    fn default_savings_rate_matches_target_norm() {
        let norm = DEFAULT_SETTINGS
            .iter()
            .find(|(key, _)| *key == "savings.target_norm_bp")
            .unwrap();
        assert_eq!(norm.1, DEFAULT_SAVINGS_RATE_BP.to_string());
    }

    #[test]
    fn default_settings_are_valid_json_with_unique_keys() {
        let mut keys = std::collections::BTreeSet::new();
        for (key, value) in DEFAULT_SETTINGS {
            serde_json::from_str::<serde_json::Value>(value).unwrap();
            assert!(keys.insert(key), "duplicate key {key}");
        }
    }
}

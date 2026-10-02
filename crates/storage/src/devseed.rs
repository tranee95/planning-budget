//! Заливка готового `DataSet` в пустую базу: dev-сид из `seed-2026.json` и тесты.

use budget_core::DataSet;
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::categories::kind_str;
use crate::incomes::status_str as income_status_str;
use crate::transactions::status_str as tx_status_str;
use crate::{Db, StorageError, stamp};

impl Db {
    /// Заменяет категории, лимиты, проценты плана и настройки расчётов данными из `data` и добавляет
    /// траты и доходы с источником `seed`. Идентификаторы сохраняются.
    ///
    /// # Errors
    /// `Conflict("seed.not_empty")`, если в базе уже есть траты или доходы (в том числе
    /// удалённые): заливка не должна ничего затирать.
    pub fn load_dataset(&mut self, data: &DataSet, now: DateTime<Utc>) -> Result<(), StorageError> {
        let used: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM transactions) OR EXISTS (SELECT 1 FROM incomes)",
            [],
            |r| r.get(0),
        )?;
        if used {
            return Err(StorageError::Conflict("seed.not_empty"));
        }
        let stamp = stamp(now);
        let tx = self.conn.transaction()?;
        tx.execute_batch("DELETE FROM categories; DELETE FROM savings_plan_overrides;")?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO categories (id, name, kind, color, sort_order, note, archived_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            )?;
            for c in &data.categories {
                insert.execute(params![
                    c.id.0,
                    c.name,
                    kind_str(c.kind),
                    c.color,
                    c.sort_order,
                    c.note,
                    c.archived.then_some(&stamp),
                    stamp
                ])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT INTO category_limits (category_id, valid_from, amount) VALUES (?1, ?2, ?3)",
            )?;
            for l in &data.limits {
                insert.execute(params![
                    l.category_id.0,
                    l.valid_from.to_string(),
                    l.amount.kopecks()
                ])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT INTO savings_category_rates (category_id, valid_from, rate_bp) VALUES (?1, ?2, ?3)",
            )?;
            for r in &data.savings_rates {
                insert.execute(params![r.category_id.0, r.valid_from.to_string(), r.rate.0])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT INTO savings_plan_overrides (month, category_id, rate_bp) VALUES (?1, ?2, ?3)",
            )?;
            for ((month, category), rate) in &data.savings_overrides {
                insert.execute(params![month.to_string(), category.0, rate.0])?;
            }
            let s = &data.settings;
            let mut insert = tx.prepare_cached(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            )?;
            let values: [(&str, String); 8] = [
                ("savings.target_min_bp", s.savings_min.0.to_string()),
                ("savings.target_norm_bp", s.savings_norm.0.to_string()),
                ("savings.target_max_bp", s.savings_max.0.to_string()),
                ("bonds.rate_bp", s.bonds_rate.0.to_string()),
                ("bonds.coupon_tax_bp", s.bonds_coupon_tax.0.to_string()),
                (
                    "bonds.initial_balance",
                    s.bonds_initial_balance.kopecks().to_string(),
                ),
                (
                    "bonds.initial_month",
                    format!("\"{}\"", s.bonds_initial_month),
                ),
                ("ui.weeks_per_month", s.weeks_per_month.to_string()),
            ];
            for (key, value) in values {
                insert.execute(params![key, value])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT INTO transactions
                     (id, month, category_id, title, amount, status, source, sort_key, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'seed',
                         (SELECT COALESCE(max(sort_key) + 1, 0) FROM transactions
                          WHERE month = ?2 AND category_id = ?3),
                         ?7, ?7)",
            )?;
            for t in &data.transactions {
                insert.execute(params![
                    t.id.0,
                    t.month.to_string(),
                    t.category_id.0,
                    t.title,
                    t.amount.kopecks(),
                    tx_status_str(t.status),
                    stamp
                ])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT INTO incomes (id, month, source_name, amount, status, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'seed', ?6, ?6)",
            )?;
            for i in &data.incomes {
                insert.execute(params![
                    i.id.0,
                    i.month.to_string(),
                    i.source_name,
                    i.amount.kopecks(),
                    income_status_str(i.status),
                    stamp
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

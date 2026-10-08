//! Заливка готового `DataSet` в пустую базу: dev-сид из `seed-2026.json` и тесты.

use crate::categories::kind_str;
use crate::incomes::status_str as income_status_str;
use crate::transactions::status_str as tx_status_str;
use crate::writes::{
    NewCategory, NewIncome, NewTransaction, insert_category, insert_income, insert_limit,
    insert_override, insert_rate, insert_transaction, upsert_savings_params, upsert_setting,
};
use crate::{Db, StorageError, stamp};
use chrono::{DateTime, Utc};
use planning_budget_core::{DataSet, Money};

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
            for c in &data.categories {
                insert_category(
                    &tx,
                    &NewCategory {
                        id: Some(c.id.0),
                        name: &c.name,
                        kind: kind_str(c.kind),
                        color: &c.color,
                        sort_order: i64::from(c.sort_order),
                        note: c.note.as_deref(),
                        archived_at: c.archived.then_some(stamp.as_str()),
                        stamp: &stamp,
                    },
                )?;
            }
            for l in &data.limits {
                if let Some(amount) = l.amount {
                    insert_limit(
                        &tx,
                        l.category_id.0,
                        &l.valid_from.to_string(),
                        amount.kopecks(),
                    )?;
                }
            }
            for r in &data.savings_rates {
                insert_rate(
                    &tx,
                    r.category_id.0,
                    &r.valid_from.to_string(),
                    i64::from(r.rate.0),
                    r.fixed_amount.map(Money::kopecks),
                )?;
            }
            for (category, p) in &data.savings_params {
                upsert_savings_params(
                    &tx,
                    category.0,
                    i64::from(p.annual_rate.0),
                    i64::from(p.tax.0),
                    p.initial_balance.kopecks(),
                    &p.initial_month.to_string(),
                )?;
            }
            for ((month, category), rate) in &data.savings_overrides {
                insert_override(&tx, &month.to_string(), category.0, i64::from(rate.0))?;
            }
            let s = &data.settings;
            let values: [(&str, String); 4] = [
                ("savings.target_min_bp", s.savings_min.0.to_string()),
                ("savings.target_norm_bp", s.savings_norm.0.to_string()),
                ("savings.target_max_bp", s.savings_max.0.to_string()),
                ("ui.weeks_per_month", s.weeks_per_month.to_string()),
            ];
            for (key, value) in values {
                upsert_setting(&tx, key, &value)?;
            }
            for t in &data.transactions {
                insert_transaction(
                    &tx,
                    &NewTransaction {
                        id: Some(t.id.0),
                        month: &t.month.to_string(),
                        category_id: t.category_id.0,
                        title: &t.title,
                        amount: t.amount.kopecks(),
                        status: tx_status_str(t.status),
                        source: "seed",
                        stamp: &stamp,
                    },
                )?;
            }
            for i in &data.incomes {
                insert_income(
                    &tx,
                    &NewIncome {
                        id: Some(i.id.0),
                        month: &i.month.to_string(),
                        source_name: &i.source_name,
                        amount: i.amount.kopecks(),
                        status: income_status_str(i.status),
                        source: "seed",
                        stamp: &stamp,
                    },
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

//! Запись разобранной старой таблицы в базу.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use planning_budget_core::{CategoryKind, LegacyBook};
use rusqlite::{OptionalExtension as _, params};

use crate::categories::{kind_from_str, kind_str};
use crate::incomes::status_str as income_status_str;
use crate::seed::PALETTE;
use crate::transactions::status_str as tx_status_str;
use crate::writes::{
    NewCategory, NewIncome, NewTransaction, ensure_savings_params, insert_category, insert_income,
    insert_rate_if_none, insert_transaction, upsert_limit, upsert_savings_params, upsert_setting,
};
use crate::{Db, StorageError, stamp};

/// Что сделал перенос. Названия категорий — данные пользователя: отчёт показывается в UI
/// и в лог не попадает.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LegacyReport {
    pub categories_created: u32,
    pub categories_matched: u32,
    /// Категории, у которых тип в файле отличается от типа в базе: тип не меняется.
    pub kind_conflicts: Vec<String>,
    pub limits_written: u32,
    pub transactions: u32,
    pub incomes: u32,
    pub settings_applied: bool,
}

impl Db {
    /// Переносит данные старой таблицы одной транзакцией: категории (слияние по имени),
    /// лимиты с первого месяца данных, настройки, траты и доходы с источником `legacy`.
    ///
    /// Категория из файла находит существующую по имени без учёта регистра и `ё` и получает
    /// заметку из файла; новые категории создаются в конце списка. Тип существующей категории
    /// не меняется (расхождение попадает в отчёт). Строка лимита с тем же `valid_from`
    /// заменяется. Лимиты по умолчанию позже первого месяца данных заменяются лимитом из таблицы. Категориям сбережений без процента плана на первый месяц данных ставится норма.
    ///
    /// # Errors
    /// `Conflict("legacy.not_empty")`, если в базе уже есть траты или доходы: повторный перенос
    /// задвоил бы данные. `Invalid("legacy.empty")`, если в файле нет ни трат, ни доходов;
    /// `Invalid("legacy.unknown_category")`, если трата ссылается на категорию вне «Справочника».
    pub fn import_legacy(
        &mut self,
        book: &LegacyBook,
        now: DateTime<Utc>,
    ) -> Result<LegacyReport, StorageError> {
        let first_month = book
            .first_month()
            .ok_or(StorageError::Invalid("legacy.empty"))?;
        let used: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM transactions) OR EXISTS (SELECT 1 FROM incomes)",
            [],
            |r| r.get(0),
        )?;
        if used {
            return Err(StorageError::Conflict("legacy.not_empty"));
        }
        let stamp = stamp(now);
        let mut report = LegacyReport::default();
        let tx = self.conn.transaction()?;

        let mut ids: HashMap<String, i64> = HashMap::with_capacity(book.categories.len());
        for category in &book.categories {
            let existing: Option<(i64, String)> = tx
                .query_row(
                    "SELECT id, kind FROM categories WHERE norm(name) = norm(?1) AND archived_at IS NULL",
                    [&category.name],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let id = if let Some((id, kind)) = existing {
                report.categories_matched += 1;
                if kind_from_str(&kind)? != category.kind {
                    report.kind_conflicts.push(category.name.clone());
                }
                if category.note.is_some() {
                    tx.execute(
                        "UPDATE categories SET note = ?2, updated_at = ?3 WHERE id = ?1",
                        params![id, category.note, stamp],
                    )?;
                }
                id
            } else {
                let position: i64 = tx.query_row(
                    "SELECT COALESCE(max(sort_order) + 1, 0) FROM categories",
                    [],
                    |r| r.get(0),
                )?;
                let color = PALETTE
                    .get(
                        usize::try_from(position).map_err(|_| StorageError::Corrupt)?
                            % PALETTE.len(),
                    )
                    .copied()
                    .unwrap_or("#6C7480");
                insert_category(
                    &tx,
                    &NewCategory {
                        id: None,
                        name: &category.name,
                        kind: kind_str(category.kind),
                        color,
                        sort_order: position,
                        note: category.note.as_deref(),
                        archived_at: None,
                        stamp: &stamp,
                    },
                )?;
                report.categories_created += 1;
                tx.last_insert_rowid()
            };
            ids.insert(category.name.clone(), id);

            if let Some(limit) = category.limit {
                // Лимиты по умолчанию, созданные вместе с хранилищем, датированы месяцем создания:
                // они перекрыли бы лимит таблицы во всех месяцах после первого.
                tx.execute(
                    "DELETE FROM category_limits WHERE category_id = ?1 AND valid_from > ?2",
                    params![id, first_month.to_string()],
                )?;
                upsert_limit(&tx, id, &first_month.to_string(), limit.kopecks())?;
                report.limits_written += 1;
            }
        }

        if let Some(s) = &book.settings {
            for (key, value) in [
                ("savings.target_min_bp", s.savings_min.0.to_string()),
                ("savings.target_norm_bp", s.savings_norm.0.to_string()),
                ("savings.target_max_bp", s.savings_max.0.to_string()),
            ] {
                upsert_setting(&tx, key, &value)?;
            }
            // Настройки облигаций относятся к первой неархивной категории сбережений (как в миграции 008).
            let first: Option<i64> = tx
                .query_row(
                    "SELECT id FROM categories WHERE kind = 'savings'
                     ORDER BY archived_at IS NOT NULL, sort_order, id LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(id) = first {
                upsert_savings_params(
                    &tx,
                    id,
                    i64::from(s.bonds_rate.0),
                    i64::from(s.bonds_coupon_tax.0),
                    s.bonds_initial_balance.kopecks(),
                    &first_month.to_string(),
                )?;
            }
            report.settings_applied = true;
        }

        ensure_savings_params(&tx, &first_month.to_string())?;

        let norm_bp: i64 = tx
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'savings.target_norm_bp'",
                [],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(StorageError::Corrupt)?;
        for category in book
            .categories
            .iter()
            .filter(|c| c.kind == CategoryKind::Savings)
        {
            let Some(id) = ids.get(&category.name) else {
                continue;
            };
            insert_rate_if_none(&tx, *id, &first_month.to_string(), norm_bp)?;
        }

        {
            for t in &book.transactions {
                let category = ids
                    .get(&t.category)
                    .ok_or(StorageError::Invalid("legacy.unknown_category"))?;
                insert_transaction(
                    &tx,
                    &NewTransaction {
                        id: None,
                        month: &t.month.to_string(),
                        category_id: *category,
                        title: &t.title,
                        amount: t.amount.kopecks(),
                        status: tx_status_str(t.status),
                        source: "legacy",
                        stamp: &stamp,
                    },
                )?;
                report.transactions += 1;
            }
            for i in &book.incomes {
                insert_income(
                    &tx,
                    &NewIncome {
                        id: None,
                        month: &i.month.to_string(),
                        source_name: &i.source_name,
                        amount: i.amount.kopecks(),
                        status: income_status_str(i.status),
                        source: "legacy",
                        stamp: &stamp,
                    },
                )?;
                report.incomes += 1;
            }
        }
        tx.commit()?;
        Ok(report)
    }
}

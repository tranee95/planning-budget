//! Полные списки по фильтру для таблиц «Расходы» и «Доходы».

use std::collections::BTreeMap;

use budget_core::query::{Filter, RecordKind, Term};
use budget_core::{Money, TagId};
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::incomes::{IncomeRecord, map_income};
use crate::search::incomes_apply;
use crate::transactions::{TransactionRecord, map_tx};
use crate::{Db, StorageError};

/// Сколько строк таблица получает за раз; `total` и `sum` считаются по всем совпадениям.
pub const LIST_LIMIT: usize = 5_000;

/// Совпавшие траты: итог по всем и строки не более `limit`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TransactionList {
    pub total: u64,
    pub sum: Money,
    pub items: Vec<TransactionRecord>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IncomeList {
    pub total: u64,
    pub sum: Money,
    pub items: Vec<IncomeRecord>,
}

impl Db {
    /// Теги страницы одним кэшируемым запросом: список id передаётся JSON-массивом, а не
    /// набором `?`, который SQLite разбирал бы заново для каждой длины списка.
    fn tags_of(&self, ids: &[i64]) -> Result<BTreeMap<i64, Vec<TagId>>, StorageError> {
        let mut out: BTreeMap<i64, Vec<TagId>> = BTreeMap::new();
        if ids.is_empty() {
            return Ok(out);
        }
        let json = serde_json::to_string(ids).map_err(|_| StorageError::Invalid("tags.ids"))?;
        let mut stmt = self.conn.prepare_cached(
            "SELECT transaction_id, tag_id FROM transaction_tags
             WHERE transaction_id IN (SELECT value FROM json_each(?1)) ORDER BY tag_id",
        )?;
        let mut rows = stmt.query([json])?;
        while let Some(row) = rows.next()? {
            out.entry(row.get(0)?).or_default().push(TagId(row.get(1)?));
        }
        Ok(out)
    }

    /// Траты по тексту и фильтру, новые сверху. Вид записи «доход» даёт пустой список.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn transaction_search(
        &self,
        text: &[Term],
        filter: &Filter,
        limit: usize,
    ) -> Result<TransactionList, StorageError> {
        if filter.record == RecordKind::Income {
            return Ok(TransactionList {
                total: 0,
                sum: Money::ZERO,
                items: Vec::new(),
            });
        }
        let fts = crate::search::fts_match(text);
        if crate::search::matches_nothing(text, fts.as_deref()) {
            return Ok(TransactionList {
                total: 0,
                sum: Money::ZERO,
                items: Vec::new(),
            });
        }
        let q = self.tx_query(filter, fts.as_deref())?;
        let (total, sum) = self.totals(&q)?;
        let sql = format!(
            "SELECT t.id, t.month, t.date, t.category_id, t.title, t.amount, t.status, \
             t.comment, t.source, t.sort_key FROM {from}{where_} \
             ORDER BY t.month DESC, t.date DESC, t.id DESC LIMIT ?",
            from = q.from,
            where_ = q.where_sql(),
        );
        let mut params = q.params;
        params.push(Value::Integer(
            i64::try_from(limit).map_err(|_| StorageError::Invalid("search.limit"))?,
        ));
        let mut items = {
            let mut stmt = self.conn.prepare_cached(&sql)?;
            let mut rows = stmt.query(params_from_iter(params.iter()))?;
            let mut items = Vec::new();
            while let Some(row) = rows.next()? {
                items.push(map_tx(row)?);
            }
            items
        };
        let ids: Vec<i64> = items.iter().map(|t| t.id.0).collect();
        let mut tags = self.tags_of(&ids)?;
        for item in &mut items {
            item.tags = tags.remove(&item.id.0).unwrap_or_default();
        }
        Ok(TransactionList { total, sum, items })
    }

    /// Доходы по тексту и фильтру. Фильтры категорий, типов, статусов и тегов их исключают.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn income_search(
        &self,
        text: &[Term],
        filter: &Filter,
        limit: usize,
    ) -> Result<IncomeList, StorageError> {
        if !incomes_apply(filter) {
            return Ok(IncomeList {
                total: 0,
                sum: Money::ZERO,
                items: Vec::new(),
            });
        }
        let fts = crate::search::fts_match(text);
        if crate::search::matches_nothing(text, fts.as_deref()) {
            return Ok(IncomeList {
                total: 0,
                sum: Money::ZERO,
                items: Vec::new(),
            });
        }
        let q = Self::income_query(filter, fts.as_deref());
        let (total, sum) = self.totals(&q)?;
        let sql = format!(
            "SELECT i.id, i.month, i.date, i.source_name, i.amount, i.status, i.comment, \
             i.source FROM {from}{where_} ORDER BY i.month DESC, i.date DESC, i.id DESC LIMIT ?",
            from = q.from,
            where_ = q.where_sql(),
        );
        let mut params = q.params;
        params.push(Value::Integer(
            i64::try_from(limit).map_err(|_| StorageError::Invalid("search.limit"))?,
        ));
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut rows = stmt.query(params_from_iter(params.iter()))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(map_income(row)?);
        }
        Ok(IncomeList { total, sum, items })
    }
}

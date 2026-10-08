//! Глобальный поиск: FTS5 по тексту и параметризованный SQL по `Filter`.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use planning_budget_core::query::{Filter, RecordKind, Term};
use planning_budget_core::{CategoryId, IncomeId, IncomeStatus, Money, TxId, TxStatus, YearMonth};
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::incomes::status_from_str as income_status_from_str;
use crate::records::{parse_date, parse_month};
use crate::transactions::status_from_str as tx_status_from_str;
use crate::{Db, StorageError};

mod query;
mod text;

use text::{could_name_a_month, terms_match};
pub(crate) use text::{fts_match, matches_nothing};

/// Найденная трата для строки результата.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TransactionHit {
    pub id: TxId,
    pub title: String,
    pub category_id: CategoryId,
    pub category: String,
    pub month: YearMonth,
    pub date: Option<NaiveDate>,
    pub amount: Money,
    pub status: TxStatus,
}

/// Найденный доход.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IncomeHit {
    pub id: IncomeId,
    pub source_name: String,
    pub month: YearMonth,
    pub date: Option<NaiveDate>,
    pub amount: Money,
    pub status: IncomeStatus,
}

/// Группа результатов: итог по всем совпадениям и первые `items`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Group<T> {
    pub total: u64,
    pub sum: Money,
    pub items: Vec<T>,
}

impl<T> Group<T> {
    fn empty() -> Self {
        Self {
            total: 0,
            sum: Money::ZERO,
            items: Vec::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CategoryHit {
    pub id: CategoryId,
    pub name: String,
}

/// Месяц, в котором есть записи, и их число.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MonthHit {
    pub month: YearMonth,
    pub records: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SearchResult {
    pub transactions: Group<TransactionHit>,
    pub incomes: Group<IncomeHit>,
    pub categories: Vec<CategoryHit>,
    pub months: Vec<MonthHit>,
}

/// Фильтр по категориям, типам, статусам и тегам относится только к тратам.
pub(crate) fn incomes_apply(filter: &Filter) -> bool {
    filter.record != RecordKind::Expense
        && filter.categories.is_empty()
        && filter.kinds.is_empty()
        && filter.statuses.is_empty()
        && filter.tags.is_empty()
}

pub(crate) fn to_u64(n: i64) -> Result<u64, StorageError> {
    u64::try_from(n).map_err(|_| StorageError::Corrupt)
}

impl Db {
    fn transaction_group(
        &self,
        filter: &Filter,
        fts: Option<&str>,
        limit: i64,
    ) -> Result<Group<TransactionHit>, StorageError> {
        let q = self.tx_query(filter, fts)?;
        let (total, sum) = self.totals(&q)?;
        let rank = if fts.is_some() {
            "bm25(search_fts), "
        } else {
            ""
        };
        let sql = format!(
            "SELECT t.id, t.title, t.category_id, c.name, t.month, t.date, t.amount, t.status \
             FROM {from}{where_} ORDER BY {rank}t.month DESC, t.date DESC, t.id DESC LIMIT ?",
            from = q.from,
            where_ = q.where_sql(),
        );
        let mut params = q.params;
        params.push(Value::Integer(limit));
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut rows = stmt.query(params_from_iter(params.iter()))?;
        let mut items = Vec::new();
        while let Some(r) = rows.next()? {
            let month: String = r.get(4)?;
            let date: Option<String> = r.get(5)?;
            let status: String = r.get(7)?;
            items.push(TransactionHit {
                id: TxId(r.get(0)?),
                title: r.get(1)?,
                category_id: CategoryId(r.get(2)?),
                category: r.get(3)?,
                month: parse_month(&month)?,
                date: date.as_deref().map(parse_date).transpose()?,
                amount: Money::from_kopecks(r.get(6)?),
                status: tx_status_from_str(&status)?,
            });
        }
        Ok(Group { total, sum, items })
    }

    fn income_group(
        &self,
        filter: &Filter,
        fts: Option<&str>,
        limit: i64,
    ) -> Result<Group<IncomeHit>, StorageError> {
        let q = Self::income_query(filter, fts);
        let (total, sum) = self.totals(&q)?;
        let rank = if fts.is_some() {
            "bm25(search_fts), "
        } else {
            ""
        };
        let sql = format!(
            "SELECT i.id, i.source_name, i.month, i.date, i.amount, i.status \
             FROM {from}{where_} ORDER BY {rank}i.month DESC, i.date DESC, i.id DESC LIMIT ?",
            from = q.from,
            where_ = q.where_sql(),
        );
        let mut params = q.params;
        params.push(Value::Integer(limit));
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut rows = stmt.query(params_from_iter(params.iter()))?;
        let mut items = Vec::new();
        while let Some(r) = rows.next()? {
            let month: String = r.get(2)?;
            let date: Option<String> = r.get(3)?;
            let status: String = r.get(5)?;
            items.push(IncomeHit {
                id: IncomeId(r.get(0)?),
                source_name: r.get(1)?,
                month: parse_month(&month)?,
                date: date.as_deref().map(parse_date).transpose()?,
                amount: Money::from_kopecks(r.get(4)?),
                status: income_status_from_str(&status)?,
            });
        }
        Ok(Group { total, sum, items })
    }

    fn category_hits(&self, text: &[Term], limit: usize) -> Result<Vec<CategoryHit>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name FROM categories WHERE archived_at IS NULL ORDER BY sort_order, id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let name: String = r.get(1)?;
            if out.len() < limit && terms_match(text, &name) {
                out.push(CategoryHit {
                    id: CategoryId(r.get(0)?),
                    name,
                });
            }
        }
        Ok(out)
    }

    fn month_hits(&self, text: &[Term], limit: usize) -> Result<Vec<MonthHit>, StorageError> {
        if !text.iter().all(could_name_a_month) {
            return Ok(Vec::new());
        }
        // Два запроса по покрывающим индексам вместо UNION ALL: тот материализует все строки.
        let mut per_month: BTreeMap<String, u64> = BTreeMap::new();
        for sql in [
            "SELECT month, count(*) FROM v_transactions GROUP BY month",
            "SELECT month, count(*) FROM v_incomes GROUP BY month",
        ] {
            let mut stmt = self.conn.prepare_cached(sql)?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                *per_month.entry(r.get(0)?).or_default() += to_u64(r.get(1)?)?;
            }
        }
        let mut out = Vec::new();
        for (raw, records) in per_month.into_iter().rev() {
            let month = parse_month(&raw)?;
            let label = format!("{} {raw}", month.ru_name());
            if out.len() < limit && terms_match(text, &label) {
                out.push(MonthHit { month, records });
            }
        }
        Ok(out)
    }

    /// Поиск для палитры ⌘K и таблиц: траты и доходы с итогами и первыми `per_group`
    /// строками, категории и месяцы, подходящие под текст.
    ///
    /// Фильтр по категориям, типам, статусам и тегам относится только к тратам: при его
    /// наличии доходы не возвращаются. Текст ранжируется по `bm25`, затем по дате.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn search(
        &self,
        text: &[Term],
        filter: &Filter,
        per_group: usize,
    ) -> Result<SearchResult, StorageError> {
        let limit = i64::try_from(per_group).map_err(|_| StorageError::Invalid("search.limit"))?;
        let fts = fts_match(text);
        if matches_nothing(text, fts.as_deref()) {
            return Ok(SearchResult {
                transactions: Group::empty(),
                incomes: Group::empty(),
                categories: Vec::new(),
                months: Vec::new(),
            });
        }
        let transactions = if filter.record == RecordKind::Income {
            Group::empty()
        } else {
            self.transaction_group(filter, fts.as_deref(), limit)?
        };
        let incomes = if incomes_apply(filter) {
            self.income_group(filter, fts.as_deref(), limit)?
        } else {
            Group::empty()
        };
        let (categories, months) = if fts.is_some() {
            (
                self.category_hits(text, per_group)?,
                self.month_hits(text, per_group)?,
            )
        } else {
            (Vec::new(), Vec::new())
        };
        Ok(SearchResult {
            transactions,
            incomes,
            categories,
            months,
        })
    }
}

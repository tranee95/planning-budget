//! Глобальный поиск: FTS5 по тексту и параметризованный SQL по `Filter`.

use std::collections::BTreeMap;

use budget_core::query::{Filter, RecordKind, Source, Term};
use budget_core::{CategoryId, IncomeId, IncomeStatus, Money, TxId, TxStatus, YearMonth};
use chrono::NaiveDate;
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::categories::kind_str;
use crate::incomes::status_from_str as income_status_from_str;
use crate::migrations::normalize;
use crate::records::{RecordSource, parse_date, parse_month};
use crate::transactions::status_from_str as tx_status_from_str;
use crate::{Db, StorageError};

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

/// Собираемый запрос: условия и параметры идут парами, пользовательский ввод — только в
/// параметры.
pub(crate) struct Query {
    pub(crate) from: String,
    /// Колонка суммы в `from`: `t.amount` для трат, `i.amount` для доходов.
    pub(crate) amount: &'static str,
    conds: Vec<String>,
    pub(crate) params: Vec<Value>,
}

impl Query {
    fn new(from: &str, amount: &'static str) -> Self {
        Self {
            from: from.to_owned(),
            amount,
            conds: Vec::new(),
            params: Vec::new(),
        }
    }

    fn cond(&mut self, sql: &str, params: impl IntoIterator<Item = Value>) {
        self.conds.push(sql.to_owned());
        self.params.extend(params);
    }

    pub(crate) fn where_sql(&self) -> String {
        if self.conds.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", self.conds.join(" AND "))
        }
    }
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

/// Условия, общие для трат и доходов: период, сумма, источник.
fn common_conds(q: &mut Query, filter: &Filter, alias: &str) {
    if let Some(range) = &filter.months {
        q.cond(
            &format!("{alias}.month BETWEEN ? AND ?"),
            [
                Value::Text(range.from.to_string()),
                Value::Text(range.to.to_string()),
            ],
        );
    }
    if let Some(amount) = &filter.amount {
        if let Some(min) = amount.min {
            q.cond(
                &format!("{alias}.amount >= ?"),
                [Value::Integer(min.kopecks())],
            );
        }
        if let Some(max) = amount.max {
            q.cond(
                &format!("{alias}.amount <= ?"),
                [Value::Integer(max.kopecks())],
            );
        }
    }
    if let Some(source) = filter.source {
        let source = match source {
            Source::Manual => RecordSource::Manual,
            Source::Import => RecordSource::Import,
            Source::Legacy => RecordSource::Legacy,
            Source::Seed => RecordSource::Seed,
        };
        q.cond(
            &format!("{alias}.source = ?"),
            [Value::Text(source.as_str().to_owned())],
        );
    }
}

/// Запрос FTS5: слово → `"слово"*`, фраза → `"a b"`; слова через пробел (AND).
/// Текст есть, но в нём нет ни букв, ни цифр (`???`, `-`): искать нечего, результат пуст,
/// а не «всё подряд», как при пустом запросе.
pub(crate) fn matches_nothing(text: &[Term], fts: Option<&str>) -> bool {
    !text.is_empty() && fts.is_none()
}

pub(crate) fn fts_match(text: &[Term]) -> Option<String> {
    let parts: Vec<String> = text
        .iter()
        .filter_map(|term| {
            let norm = normalize(&term.text);
            if !norm.chars().any(char::is_alphanumeric) {
                return None;
            }
            let quoted = norm.replace('"', "\"\"");
            Some(if term.phrase {
                format!("\"{quoted}\"")
            } else {
                format!("\"{quoted}\"*")
            })
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// Слова поискового текста, каждое из которых начинает слово в `haystack` (или фраза входит).
fn terms_match(text: &[Term], haystack: &str) -> bool {
    let hay = normalize(haystack);
    text.iter().all(|term| {
        let needle = normalize(&term.text);
        if term.phrase {
            hay.contains(&needle)
        } else {
            hay.split(' ').any(|word| word.starts_with(&needle))
        }
    })
}

/// Дешёвая проверка до запроса: группировка по месяцам читает всю таблицу.
fn could_name_a_month(term: &Term) -> bool {
    let needle = normalize(&term.text);
    needle.starts_with(|c: char| c.is_ascii_digit())
        || (YearMonth::new(2000, 1).is_ok()
            && (1..=12).any(|m| {
                YearMonth::new(2000, m)
                    .is_ok_and(|ym| normalize(ym.month_name_ru()).starts_with(&needle))
            }))
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
    /// Id категорий, имя которых (целиком или по слову) начинается с одной из ссылок.
    fn category_ids_by_refs(
        &self,
        refs: &[budget_core::query::CategoryRef],
    ) -> Result<Vec<i64>, StorageError> {
        if refs.is_empty() {
            return Ok(Vec::new());
        }
        let needles: Vec<String> = refs.iter().map(|r| normalize(&r.0)).collect();
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, norm(name) FROM categories")?;
        let mut rows = stmt.query([])?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            let hit = needles
                .iter()
                .any(|n| name.split(' ').any(|w| w.starts_with(n)) || name.starts_with(n));
            if hit {
                ids.push(row.get(0)?);
            }
        }
        Ok(ids)
    }

    fn tag_id(&self, name: &str) -> Result<Option<i64>, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id FROM tags WHERE norm(name) = ?1")?;
        let mut rows = stmt.query([normalize(name)])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub(crate) fn tx_query(
        &self,
        filter: &Filter,
        fts: Option<&str>,
    ) -> Result<Query, StorageError> {
        let mut q = Query::new(
            if fts.is_some() {
                "search_fts JOIN v_transactions t ON t.id = (search_fts.rowid >> 1) \
             JOIN categories c ON c.id = t.category_id"
            } else {
                "v_transactions t JOIN categories c ON c.id = t.category_id"
            },
            "t.amount",
        );
        if let Some(fts) = fts {
            q.cond(
                "search_fts MATCH ? AND (search_fts.rowid & 1) = 0",
                [Value::Text(fts.to_owned())],
            );
        }
        common_conds(&mut q, filter, "t");
        if !filter.categories.is_empty() {
            let ids = self.category_ids_by_refs(&filter.categories)?;
            if ids.is_empty() {
                q.cond("0", []);
            } else {
                q.cond(
                    &format!("t.category_id IN ({})", placeholders(ids.len())),
                    ids.into_iter().map(Value::Integer),
                );
            }
        }
        let excluded = self.category_ids_by_refs(&filter.exclude_categories)?;
        if !excluded.is_empty() {
            q.cond(
                &format!("t.category_id NOT IN ({})", placeholders(excluded.len())),
                excluded.into_iter().map(Value::Integer),
            );
        }
        if !filter.kinds.is_empty() {
            q.cond(
                &format!("c.kind IN ({})", placeholders(filter.kinds.len())),
                filter
                    .kinds
                    .iter()
                    .map(|k| Value::Text(kind_str(*k).to_owned())),
            );
        }
        if !filter.statuses.is_empty() {
            q.cond(
                &format!("t.status IN ({})", placeholders(filter.statuses.len())),
                filter
                    .statuses
                    .iter()
                    .map(|s| Value::Text(crate::transactions::status_str(*s).to_owned())),
            );
        }
        for tag in &filter.tags {
            match self.tag_id(tag)? {
                Some(id) => q.cond(
                    "t.id IN (SELECT tt.transaction_id FROM transaction_tags tt \
                     WHERE tt.tag_id = ?)",
                    [Value::Integer(id)],
                ),
                None => q.cond("0", []),
            }
        }
        for tag in &filter.exclude_tags {
            if let Some(id) = self.tag_id(tag)? {
                q.cond(
                    "NOT EXISTS (SELECT 1 FROM transaction_tags tt \
                     WHERE tt.transaction_id = t.id AND tt.tag_id = ?)",
                    [Value::Integer(id)],
                );
            }
        }
        Ok(q)
    }

    pub(crate) fn income_query(filter: &Filter, fts: Option<&str>) -> Query {
        let mut q = Query::new(
            if fts.is_some() {
                "search_fts JOIN v_incomes i ON i.id = (search_fts.rowid >> 1)"
            } else {
                "v_incomes i"
            },
            "i.amount",
        );
        if let Some(fts) = fts {
            q.cond(
                "search_fts MATCH ? AND (search_fts.rowid & 1) = 1",
                [Value::Text(fts.to_owned())],
            );
        }
        common_conds(&mut q, filter, "i");
        q
    }

    pub(crate) fn totals(&self, q: &Query) -> Result<(u64, Money), StorageError> {
        let sql = format!(
            "SELECT count(*), coalesce(sum({amount}), 0) FROM {from}{where_}",
            amount = q.amount,
            from = q.from,
            where_ = q.where_sql(),
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let (count, sum): (i64, i64) = stmt.query_row(params_from_iter(q.params.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
        Ok((to_u64(count)?, Money::from_kopecks(sum)))
    }

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

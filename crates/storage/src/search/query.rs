//! Сборка SQL по `Filter`: условия и параметры идут парами.

use planning_budget_core::Money;
use planning_budget_core::query::{Filter, Source};
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::categories::kind_str;
use crate::migrations::normalize;
use crate::records::RecordSource;
use crate::{Db, StorageError};

use super::to_u64;

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
    pub(super) fn new(from: &str, amount: &'static str) -> Self {
        Self {
            from: from.to_owned(),
            amount,
            conds: Vec::new(),
            params: Vec::new(),
        }
    }

    pub(super) fn cond(&mut self, sql: &str, params: impl IntoIterator<Item = Value>) {
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

impl Db {
    /// Id категорий, имя которых (целиком или по слову) начинается с одной из ссылок.
    fn category_ids_by_refs(
        &self,
        refs: &[planning_budget_core::query::CategoryRef],
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
}

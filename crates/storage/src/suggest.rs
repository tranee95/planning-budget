//! Подсказки быстрого добавления по всей истории трат.

use planning_budget_core::query::Term;
use planning_budget_core::{CategoryId, YearMonth};
use rusqlite::params;

use crate::migrations::normalize;
use crate::search::{fts_match, to_u64};
use crate::{Db, StorageError};

/// Наименование из истории: категория, в которой его чаще всего вводили, и число таких трат.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TitleSuggestion {
    pub title: String,
    pub category_id: CategoryId,
    pub uses: u64,
}

impl Db {
    /// Наименования, в которых каждое слово запроса начинает слово названия. Чаще
    /// встречающиеся — выше; точное совпадение с запросом не предлагается.
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn title_suggestions(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<TitleSuggestion>, StorageError> {
        let words: Vec<Term> = query
            .split_whitespace()
            .map(|w| Term {
                text: w.to_owned(),
                phrase: false,
            })
            .collect();
        let Some(fts) = fts_match(&words) else {
            return Ok(Vec::new());
        };
        let wanted = normalize(query);
        let mut stmt = self.conn.prepare_cached(
            "SELECT t.title, t.category_id, count(*) AS uses
             FROM search_fts JOIN v_transactions t ON t.id = (search_fts.rowid >> 1)
             WHERE search_fts MATCH ?1 AND (search_fts.rowid & 1) = 0
             GROUP BY norm(t.title), t.category_id
             ORDER BY uses DESC, max(t.id) DESC LIMIT ?2",
        )?;
        // Колонка `title` — только название траты, без категории, тегов и комментария.
        let mut rows = stmt.query(params![
            format!("title : ({fts})"),
            i64::try_from(limit.saturating_mul(4)).unwrap_or(i64::MAX)
        ])?;
        let mut out: Vec<TitleSuggestion> = Vec::new();
        while let Some(row) = rows.next()? {
            let title: String = row.get(0)?;
            let key = normalize(&title);
            if key == wanted || out.iter().any(|s| normalize(&s.title) == key) {
                continue;
            }
            out.push(TitleSuggestion {
                title,
                category_id: CategoryId(row.get(1)?),
                uses: to_u64(row.get(2)?)?,
            });
            if out.len() == limit {
                break;
            }
        }
        Ok(out)
    }

    /// Сколько трат в каждой категории начиная с месяца `since`: для порядка чипов категорий.
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn category_usage(&self, since: YearMonth) -> Result<Vec<(CategoryId, u64)>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT category_id, count(*) FROM v_transactions WHERE month >= ?1
             GROUP BY category_id ORDER BY count(*) DESC, category_id",
        )?;
        let mut rows = stmt.query([since.to_string()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((CategoryId(row.get(0)?), to_u64(row.get(1)?)?));
        }
        Ok(out)
    }
}

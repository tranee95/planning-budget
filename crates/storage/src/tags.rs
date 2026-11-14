//! Теги трат.

use planning_budget_core::query::{mentions_tag, rename_tag_in_query};
use planning_budget_core::{Tag, TagId};
use rusqlite::params;
use std::collections::HashMap;

use crate::{Db, StorageError};

fn validate_name(name: &str) -> Result<&str, StorageError> {
    let name = name.trim();
    if name.is_empty() {
        Err(StorageError::Invalid("tag.name_empty"))
    } else {
        Ok(name)
    }
}

fn name_conflict(err: rusqlite::Error) -> StorageError {
    let err = StorageError::from(err);
    if err.is_constraint() {
        StorageError::Conflict("tag.name_taken")
    } else {
        err
    }
}

/// Тег с числом трат и ссылками на него из сохранённых запросов.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TagUsage {
    pub tag: Tag,
    /// Неудалённые траты с тегом.
    pub uses: u32,
    /// Сохранённые фильтры и карточки графиков с `#тегом` в запросе.
    pub refs: u32,
}

impl Db {
    /// Все теги по имени.
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn tags(&self) -> Result<Vec<Tag>, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, name FROM tags ORDER BY norm(name), id")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Tag {
                id: TagId(row.get(0)?),
                name: row.get(1)?,
            });
        }
        Ok(out)
    }

    /// Все теги с числом неудалённых трат и числом сохранённых фильтров и графиков, в запросе
    /// которых тег назван целиком (`#имя`).
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn tags_usage(&self) -> Result<Vec<TagUsage>, StorageError> {
        let mut uses: HashMap<TagId, u32> = HashMap::new();
        let mut stmt = self.conn.prepare_cached(
            "SELECT tt.tag_id, COUNT(*) FROM transaction_tags tt \
             JOIN transactions t ON t.id = tt.transaction_id \
             WHERE t.deleted_at IS NULL GROUP BY tt.tag_id",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            uses.insert(TagId(row.get(0)?), row.get(1)?);
        }
        let queries = self.stored_queries()?;
        Ok(self
            .tags()?
            .into_iter()
            .map(|tag| {
                let refs = queries
                    .iter()
                    .filter(|q| mentions_tag(q, &tag.name))
                    .count();
                TagUsage {
                    uses: uses.get(&tag.id).copied().unwrap_or(0),
                    refs: u32::try_from(refs).unwrap_or(u32::MAX),
                    tag,
                }
            })
            .collect())
    }

    /// Строки запросов, которые хранит бюджет: сохранённые фильтры и фильтры карточек графиков.
    fn stored_queries(&self) -> Result<Vec<String>, StorageError> {
        let mut out = Vec::new();
        let mut stmt = self
            .conn
            .prepare_cached("SELECT query FROM saved_filters")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            out.push(row.get::<_, String>(0)?);
        }
        let mut stmt = self.conn.prepare_cached("SELECT spec FROM charts")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            // Нечитаемое описание графика не должно ломать список тегов: пропускаем его.
            let Ok(spec) = serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(0)?)
            else {
                continue;
            };
            if let Some(filter) = spec.get("filter").and_then(serde_json::Value::as_str) {
                out.push(filter.to_owned());
            }
        }
        Ok(out)
    }

    /// Создаёт тег.
    ///
    /// # Errors
    /// `Invalid` при пустом имени; `Conflict`, если такое имя уже есть (без учёта регистра и `ё`).
    pub fn tag_create(&mut self, name: &str) -> Result<Tag, StorageError> {
        let name = validate_name(name)?;
        self.conn
            .execute("INSERT INTO tags (name) VALUES (?1)", [name])
            .map_err(name_conflict)?;
        Ok(Tag {
            id: TagId(self.conn.last_insert_rowid()),
            name: name.to_owned(),
        })
    }

    /// Переименовывает тег; индекс поиска обновляют триггеры. Условия `#старое имя` в сохранённых
    /// фильтрах и фильтрах карточек графиков переписываются на новое имя в той же транзакции.
    ///
    /// # Errors
    /// `NotFound`, `Invalid`, `Conflict`.
    pub fn tag_rename(&mut self, id: TagId, name: &str) -> Result<Tag, StorageError> {
        let name = validate_name(name)?;
        let tx = self.conn.transaction()?;
        let old: String = tx
            .query_row("SELECT name FROM tags WHERE id = ?1", [id.0], |r| r.get(0))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
                other => other.into(),
            })?;
        tx.execute(
            "UPDATE tags SET name = ?2 WHERE id = ?1",
            params![id.0, name],
        )
        .map_err(name_conflict)?;
        rewrite_stored_queries(&tx, &old, name)?;
        tx.commit()?;
        Ok(Tag {
            id,
            name: name.to_owned(),
        })
    }

    /// Удаляет тег и снимает его со всех трат.
    ///
    /// # Errors
    /// `NotFound`, если тега нет.
    pub fn tag_delete(&mut self, id: TagId) -> Result<(), StorageError> {
        let changed = self
            .conn
            .execute("DELETE FROM tags WHERE id = ?1", [id.0])?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

/// Переписывает `#from` на `#to` в сохранённых фильтрах и в `filter` описаний графиков.
fn rewrite_stored_queries(
    tx: &rusqlite::Transaction<'_>,
    from: &str,
    to: &str,
) -> Result<(), StorageError> {
    let mut filters = Vec::new();
    {
        let mut stmt = tx.prepare("SELECT id, query FROM saved_filters")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            filters.push((row.get::<_, i64>(0)?, row.get::<_, String>(1)?));
        }
    }
    for (id, query) in filters {
        if let Some(new) = rename_tag_in_query(&query, from, to) {
            tx.execute(
                "UPDATE saved_filters SET query = ?1 WHERE id = ?2",
                params![new, id],
            )?;
        }
    }
    let mut charts = Vec::new();
    {
        let mut stmt = tx.prepare("SELECT id, spec FROM charts")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            charts.push((row.get::<_, i64>(0)?, row.get::<_, String>(1)?));
        }
    }
    for (id, json) in charts {
        let Ok(mut spec) = serde_json::from_str::<serde_json::Value>(&json) else {
            continue;
        };
        let Some(filter) = spec.get("filter").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if let Some(new) = rename_tag_in_query(filter, from, to) {
            if let Some(slot) = spec.get_mut("filter") {
                *slot = serde_json::Value::String(new);
            }
            tx.execute(
                "UPDATE charts SET spec = ?1 WHERE id = ?2",
                params![spec.to_string(), id],
            )?;
        }
    }
    Ok(())
}

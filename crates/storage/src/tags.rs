//! Теги трат.

use planning_budget_core::{Tag, TagId};
use rusqlite::params;

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

    /// Переименовывает тег; индекс поиска обновляют триггеры.
    ///
    /// # Errors
    /// `NotFound`, `Invalid`, `Conflict`.
    pub fn tag_rename(&mut self, id: TagId, name: &str) -> Result<Tag, StorageError> {
        let name = validate_name(name)?;
        let changed = self
            .conn
            .execute(
                "UPDATE tags SET name = ?2 WHERE id = ?1",
                params![id.0, name],
            )
            .map_err(name_conflict)?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
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

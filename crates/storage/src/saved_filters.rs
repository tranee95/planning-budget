//! Сохранённые фильтры: имя, экран и строка запроса языка (`saved_filters`).

use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::{Db, StorageError, stamp};

const MAX_NAME_CHARS: usize = 60;
const MAX_QUERY_CHARS: usize = 500;

/// Экран, к которому относится фильтр (колонка `screen`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterScreen {
    Expenses,
    Incomes,
}

impl FilterScreen {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Expenses => "expenses",
            Self::Incomes => "incomes",
        }
    }

    /// Неизвестное значение (его не пропустит `CHECK`) читается как расходы.
    fn from_db(text: &str) -> Self {
        if text == "incomes" {
            Self::Incomes
        } else {
            Self::Expenses
        }
    }
}

/// Сохранённый запрос. Хранится строкой: разбор при применении ведёт `core::query`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SavedFilter {
    pub id: i64,
    pub name: String,
    pub query: String,
    pub screen: FilterScreen,
}

impl Db {
    /// Все фильтры по имени.
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn saved_filters(&self) -> Result<Vec<SavedFilter>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, query, screen FROM saved_filters ORDER BY norm(name), id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(SavedFilter {
                id: row.get(0)?,
                name: row.get(1)?,
                query: row.get(2)?,
                screen: FilterScreen::from_db(&row.get::<_, String>(3)?),
            });
        }
        Ok(out)
    }

    /// Сохраняет фильтр; фильтр с тем же именем (без учёта регистра и `ё`) на том же
    /// экране получает новый запрос.
    ///
    /// # Errors
    /// `Invalid` при пустом или слишком длинном имени и запросе.
    pub fn saved_filter_save(
        &mut self,
        name: &str,
        query: &str,
        screen: FilterScreen,
        now: DateTime<Utc>,
    ) -> Result<SavedFilter, StorageError> {
        let name = name.trim();
        let query = query.trim();
        if name.is_empty() {
            return Err(StorageError::Invalid("filter.name_empty"));
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(StorageError::Invalid("filter.name_too_long"));
        }
        if query.is_empty() || query.chars().count() > MAX_QUERY_CHARS {
            return Err(StorageError::Invalid("filter.query_invalid"));
        }
        let tx = self.conn.transaction()?;
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM saved_filters WHERE norm(name) = norm(?1) AND screen = ?2",
                params![name, screen.as_str()],
                |r| r.get(0),
            )
            .ok();
        let id = match existing {
            Some(id) => {
                tx.execute(
                    "UPDATE saved_filters SET name = ?1, query = ?2 WHERE id = ?3",
                    params![name, query, id],
                )?;
                id
            }
            None => {
                tx.execute(
                    "INSERT INTO saved_filters (name, query, screen, created_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![name, query, screen.as_str(), stamp(now)],
                )?;
                tx.last_insert_rowid()
            }
        };
        tx.commit()?;
        Ok(SavedFilter {
            id,
            name: name.to_owned(),
            query: query.to_owned(),
            screen,
        })
    }

    /// Удаляет фильтр.
    ///
    /// # Errors
    /// [`StorageError::NotFound`], если такого фильтра нет.
    pub fn saved_filter_delete(&mut self, id: i64) -> Result<(), StorageError> {
        let n = self
            .conn
            .execute("DELETE FROM saved_filters WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

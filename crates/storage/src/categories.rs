//! Категории: список, создание, правка, порядок, архив, удаление.

use budget_core::{Category, CategoryId, CategoryKind, YearMonth};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension as _, Row, params};

use crate::{Db, StorageError, stamp};

const SELECT_CATEGORY: &str =
    "SELECT id, name, kind, color, sort_order, note, archived_at FROM categories";

/// Поля новой категории. Тип после создания не меняется: к нему привязаны лимиты
/// или проценты плана.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NewCategory {
    pub name: String,
    pub kind: CategoryKind,
    pub color: String,
    pub note: Option<String>,
}

/// Изменяемые поля категории; `None` — не менять. `note: Some(None)` очищает заметку.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct CategoryPatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub note: Option<Option<String>>,
}

pub(crate) fn kind_str(kind: CategoryKind) -> &'static str {
    match kind {
        CategoryKind::Mandatory => "mandatory",
        CategoryKind::Wants => "wants",
        CategoryKind::Savings => "savings",
        CategoryKind::Loans => "loans",
    }
}

pub(crate) fn kind_from_str(s: &str) -> Result<CategoryKind, StorageError> {
    match s {
        "mandatory" => Ok(CategoryKind::Mandatory),
        "wants" => Ok(CategoryKind::Wants),
        "savings" => Ok(CategoryKind::Savings),
        "loans" => Ok(CategoryKind::Loans),
        _ => Err(StorageError::Corrupt),
    }
}

fn map_category(r: &Row<'_>) -> Result<Category, StorageError> {
    let sort_order: i64 = r.get("sort_order")?;
    let kind: String = r.get("kind")?;
    let archived_at: Option<String> = r.get("archived_at")?;
    Ok(Category {
        id: CategoryId(r.get("id")?),
        name: r.get("name")?,
        kind: kind_from_str(&kind)?,
        color: r.get("color")?,
        sort_order: i32::try_from(sort_order).map_err(|_| StorageError::Corrupt)?,
        note: r.get("note")?,
        archived: archived_at.is_some(),
    })
}

fn validate_name(name: &str) -> Result<&str, StorageError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(StorageError::Invalid("category.name_empty"));
    }
    Ok(name)
}

fn validate_color(color: &str) -> Result<(), StorageError> {
    let hex = color.strip_prefix('#').unwrap_or_default();
    if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(StorageError::Invalid("category.color_invalid"))
    }
}

/// Имя занято активной категорией — единственный вид нарушения уникальности при записи.
fn name_conflict(err: StorageError) -> StorageError {
    if err.is_constraint() {
        StorageError::Conflict("category.name_taken")
    } else {
        err
    }
}

impl Db {
    /// Категории по `sort_order`. Архивные — только при `include_archived`.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn categories(&self, include_archived: bool) -> Result<Vec<Category>, StorageError> {
        let sql = format!(
            "{SELECT_CATEGORY} {} ORDER BY sort_order, id",
            if include_archived {
                ""
            } else {
                "WHERE archived_at IS NULL"
            }
        );
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_category(row)?);
        }
        Ok(out)
    }

    /// Категория по id (в том числе архивная).
    ///
    /// # Errors
    /// [`StorageError::NotFound`], если такой категории нет.
    pub fn category(&self, id: CategoryId) -> Result<Category, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{SELECT_CATEGORY} WHERE id = ?1"))?;
        let mut rows = stmt.query([id.0])?;
        match rows.next()? {
            Some(row) => map_category(row),
            None => Err(StorageError::NotFound),
        }
    }

    /// Создаёт категорию в конце списка.
    ///
    /// # Errors
    /// `Invalid` при пустом имени или цвете не вида `#RRGGBB`; `Conflict`, если имя занято.
    pub fn category_create(
        &mut self,
        input: &NewCategory,
        now: DateTime<Utc>,
    ) -> Result<Category, StorageError> {
        let name = validate_name(&input.name)?;
        validate_color(&input.color)?;
        let stamp = stamp(now);
        let tx = self.conn.transaction()?;
        let next_order: i64 = tx.query_row(
            "SELECT COALESCE(max(sort_order) + 1, 0) FROM categories",
            [],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO categories (name, kind, color, sort_order, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                name,
                kind_str(input.kind),
                input.color,
                next_order,
                input.note,
                stamp
            ],
        )
        .map_err(|e| name_conflict(e.into()))?;
        let id = CategoryId(tx.last_insert_rowid());
        tx.commit()?;
        self.category(id)
    }

    /// Меняет имя, цвет или заметку.
    ///
    /// # Errors
    /// `NotFound`, `Invalid`, `Conflict` (имя занято другой активной категорией).
    pub fn category_update(
        &mut self,
        id: CategoryId,
        patch: &CategoryPatch,
        now: DateTime<Utc>,
    ) -> Result<Category, StorageError> {
        let current = self.category(id)?;
        let name = match &patch.name {
            Some(name) => validate_name(name)?.to_owned(),
            None => current.name,
        };
        let color = match &patch.color {
            Some(color) => {
                validate_color(color)?;
                color.clone()
            }
            None => current.color,
        };
        let note = patch.note.clone().unwrap_or(current.note);
        self.conn
            .execute(
                "UPDATE categories SET name = ?2, color = ?3, note = ?4, updated_at = ?5 WHERE id = ?1",
                params![id.0, name, color, note, stamp(now)],
            )
            .map_err(|e| name_conflict(e.into()))?;
        self.category(id)
    }

    /// Задаёт порядок: `ids` — все категории в новом порядке сверху вниз.
    ///
    /// # Errors
    /// `Invalid`, если список не совпадает с набором неархивных категорий.
    pub fn categories_reorder(
        &mut self,
        ids: &[CategoryId],
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        let mut active: Vec<i64> = {
            let mut stmt = tx.prepare("SELECT id FROM categories WHERE archived_at IS NULL")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<Result<_, _>>()?
        };
        let mut requested: Vec<i64> = ids.iter().map(|id| id.0).collect();
        active.sort_unstable();
        requested.sort_unstable();
        if active != requested {
            return Err(StorageError::Invalid("category.reorder_mismatch"));
        }
        let stamp = stamp(now);
        {
            let mut stmt = tx.prepare_cached(
                "UPDATE categories SET sort_order = ?2, updated_at = ?3 WHERE id = ?1",
            )?;
            for (position, id) in ids.iter().enumerate() {
                let position = i64::try_from(position).map_err(|_| StorageError::Corrupt)?;
                stmt.execute(params![id.0, position, stamp])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Архивирует категорию: траты остаются, в списках выбора она скрыта. У категории
    /// сбережений с `month` процент плана становится нулевым, чтобы она не входила в план.
    ///
    /// # Errors
    /// `NotFound`; `Conflict`, если это последняя активная категория сбережений, а в году
    /// `month` есть сбережения.
    pub fn category_archive(
        &mut self,
        id: CategoryId,
        month: YearMonth,
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let category = self.category(id)?;
        if category.archived {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        if category.kind == CategoryKind::Savings {
            let others: i64 = tx.query_row(
                "SELECT count(*) FROM categories
                 WHERE kind = 'savings' AND archived_at IS NULL AND id <> ?1",
                [id.0],
                |r| r.get(0),
            )?;
            if others == 0 {
                let year_prefix = format!("{:04}-%", month.year());
                let has_savings: bool = tx.query_row(
                    "SELECT EXISTS (
                         SELECT 1 FROM v_transactions t JOIN categories c ON c.id = t.category_id
                         WHERE c.kind = 'savings' AND t.month LIKE ?1)",
                    [year_prefix],
                    |r| r.get(0),
                )?;
                if has_savings {
                    return Err(StorageError::Conflict("category.last_savings"));
                }
            }
            // Строки процента и ручные значения «после» месяца архивации вернули бы категорию в план.
            tx.execute(
                "DELETE FROM savings_category_rates WHERE category_id = ?1 AND valid_from > ?2",
                params![id.0, month.to_string()],
            )?;
            tx.execute(
                "DELETE FROM savings_plan_overrides WHERE category_id = ?1 AND month >= ?2",
                params![id.0, month.to_string()],
            )?;
            tx.execute(
                "INSERT INTO savings_category_rates (category_id, valid_from, rate_bp) VALUES (?1, ?2, 0)
                 ON CONFLICT (category_id, valid_from) DO UPDATE SET rate_bp = 0",
                params![id.0, month.to_string()],
            )?;
        }
        tx.execute(
            "UPDATE categories SET archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id.0, stamp(now)],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Возвращает категорию из архива. Процент плана сбережений не восстанавливается:
    /// пользователь задаёт его заново.
    ///
    /// # Errors
    /// `NotFound`; `Conflict`, если активная категория уже занимает это имя.
    pub fn category_unarchive(
        &mut self,
        id: CategoryId,
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.category(id)?;
        self.conn
            .execute(
                "UPDATE categories SET archived_at = NULL, updated_at = ?2 WHERE id = ?1",
                params![id.0, stamp(now)],
            )
            .map_err(|e| name_conflict(e.into()))?;
        Ok(())
    }

    /// Физически удаляет категорию без трат (в том числе мягко удалённых). Лимиты и
    /// проценты плана удаляются каскадом.
    ///
    /// # Errors
    /// `NotFound`; `Conflict`, если на категорию ссылаются траты или правила импорта.
    pub fn category_delete(&mut self, id: CategoryId) -> Result<(), StorageError> {
        let used: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM transactions WHERE category_id = ?1)
                 OR EXISTS (SELECT 1 FROM import_rules WHERE category_id = ?1)",
            [id.0],
            |r| r.get(0),
        )?;
        if used {
            return Err(StorageError::Conflict("category.in_use"));
        }
        let deleted = self
            .conn
            .execute("DELETE FROM categories WHERE id = ?1", [id.0])?;
        if deleted == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Id активной категории по имени (поиск без учёта регистра и `ё`).
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn category_id_by_name(&self, name: &str) -> Result<Option<CategoryId>, StorageError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM categories WHERE norm(name) = norm(?1) AND archived_at IS NULL",
                [name],
                |r| r.get(0),
            )
            .optional()?
            .map(CategoryId))
    }
}

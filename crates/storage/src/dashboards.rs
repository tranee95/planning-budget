//! Дашборды и карточки графиков.

use std::collections::BTreeSet;

use budget_core::analytics::ChartSpec;
use chrono::{DateTime, Utc};
use rusqlite::{Transaction, params};

use crate::{Db, StorageError, stamp};

const MAX_NAME_CHARS: usize = 60;
/// Сетка из 12 колонок.
pub const GRID_COLUMNS: u8 = 12;
const MAX_CARD_HEIGHT: u8 = 12;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Dashboard {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
}

/// Положение карточки в ячейках сетки.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CardPlacement {
    pub id: i64,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChartCard {
    pub id: i64,
    pub dashboard_id: i64,
    pub spec: ChartSpec,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

fn clean_name(name: &str) -> Result<&str, StorageError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(StorageError::Invalid("dashboard.name_empty"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(StorageError::Invalid("dashboard.name_too_long"));
    }
    Ok(name)
}

fn check_size(w: u8, h: u8) -> Result<(), StorageError> {
    if !(1..=GRID_COLUMNS).contains(&w) || !(1..=MAX_CARD_HEIGHT).contains(&h) {
        return Err(StorageError::Invalid("dashboard.card_size"));
    }
    Ok(())
}

fn check_spec(spec: &ChartSpec) -> Result<String, StorageError> {
    spec.validate()
        .map_err(|e| StorageError::Invalid(e.key()))?;
    serde_json::to_string(spec).map_err(|_| StorageError::Corrupt)
}

fn card_from_row(row: &rusqlite::Row<'_>) -> Result<ChartCard, StorageError> {
    let spec: String = row.get("spec")?;
    Ok(ChartCard {
        id: row.get("id")?,
        dashboard_id: row.get("dashboard_id")?,
        spec: serde_json::from_str(&spec).map_err(|_| StorageError::Corrupt)?,
        x: row.get("x")?,
        y: row.get("y")?,
        w: row.get("w")?,
        h: row.get("h")?,
    })
}

fn load_card(tx: &Transaction<'_>, id: i64) -> Result<ChartCard, StorageError> {
    let mut stmt =
        tx.prepare_cached("SELECT id, dashboard_id, spec, x, y, w, h FROM charts WHERE id = ?1")?;
    let mut rows = stmt.query([id])?;
    match rows.next()? {
        Some(row) => card_from_row(row),
        None => Err(StorageError::NotFound),
    }
}

impl Db {
    /// Дашборды по порядку вкладок.
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn dashboards(&self) -> Result<Vec<Dashboard>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, is_default FROM dashboards ORDER BY sort_order, id",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Dashboard {
                id: row.get(0)?,
                name: row.get(1)?,
                is_default: row.get::<_, i64>(2)? != 0,
            });
        }
        Ok(out)
    }

    /// Новый пустой дашборд в конце списка вкладок.
    ///
    /// # Errors
    /// `Invalid` при пустом или слишком длинном имени.
    pub fn dashboard_create(&mut self, name: &str) -> Result<Dashboard, StorageError> {
        let name = clean_name(name)?;
        self.conn.execute(
            "INSERT INTO dashboards (name, sort_order, is_default)
             VALUES (?1, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM dashboards), 0)",
            [name],
        )?;
        Ok(Dashboard {
            id: self.conn.last_insert_rowid(),
            name: name.to_owned(),
            is_default: false,
        })
    }

    /// # Errors
    /// `Invalid` при плохом имени, `NotFound`, если дашборда нет.
    pub fn dashboard_rename(&mut self, id: i64, name: &str) -> Result<(), StorageError> {
        let name = clean_name(name)?;
        let n = self.conn.execute(
            "UPDATE dashboards SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Удаляет дашборд вместе с карточками. Последний дашборд удалить нельзя; если удалён
    /// основной, основным становится первый из оставшихся.
    ///
    /// # Errors
    /// `Invalid`, если дашборд последний; `NotFound`, если его нет.
    pub fn dashboard_delete(&mut self, id: i64) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        let total: i64 = tx.query_row("SELECT count(*) FROM dashboards", [], |r| r.get(0))?;
        if total <= 1 {
            return Err(StorageError::Invalid("dashboard.last"));
        }
        let n = tx.execute("DELETE FROM dashboards WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        tx.execute(
            "UPDATE dashboards SET is_default = 1
             WHERE id = (SELECT id FROM dashboards ORDER BY sort_order, id LIMIT 1)
               AND NOT EXISTS (SELECT 1 FROM dashboards WHERE is_default = 1)",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Карточки дашборда сверху вниз, слева направо.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённое описание графика.
    pub fn charts(&self, dashboard_id: i64) -> Result<Vec<ChartCard>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, dashboard_id, spec, x, y, w, h FROM charts
             WHERE dashboard_id = ?1 ORDER BY y, x, id",
        )?;
        let mut rows = stmt.query([dashboard_id])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(card_from_row(row)?);
        }
        Ok(out)
    }

    /// Добавляет карточку под всеми остальными.
    ///
    /// # Errors
    /// `Invalid` при недопустимом графике или размере, `NotFound`, если дашборда нет.
    pub fn chart_create(
        &mut self,
        dashboard_id: i64,
        spec: &ChartSpec,
        w: u8,
        h: u8,
        now: DateTime<Utc>,
    ) -> Result<ChartCard, StorageError> {
        check_size(w, h)?;
        let json = check_spec(spec)?;
        let tx = self.conn.transaction()?;
        let exists: i64 = tx.query_row(
            "SELECT count(*) FROM dashboards WHERE id = ?1",
            [dashboard_id],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Err(StorageError::NotFound);
        }
        let y: i64 = tx.query_row(
            "SELECT COALESCE(MAX(y + h), 0) FROM charts WHERE dashboard_id = ?1",
            [dashboard_id],
            |r| r.get(0),
        )?;
        // Строка карточки хранится в `u8`: дашборд, не влезающий в 255 ячеек, не загрузился бы.
        if y + i64::from(h) > i64::from(u8::MAX) {
            return Err(StorageError::Invalid("dashboard.full"));
        }
        let at = stamp(now);
        tx.execute(
            "INSERT INTO charts (dashboard_id, spec, x, y, w, h, created_at, updated_at)
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?6)",
            params![dashboard_id, json, y, w, h, at],
        )?;
        let card = load_card(&tx, tx.last_insert_rowid())?;
        tx.commit()?;
        Ok(card)
    }

    /// Меняет описание графика, положение не трогает.
    ///
    /// # Errors
    /// `Invalid` при недопустимом графике, `NotFound`, если карточки нет.
    pub fn chart_update(
        &mut self,
        id: i64,
        spec: &ChartSpec,
        now: DateTime<Utc>,
    ) -> Result<ChartCard, StorageError> {
        let json = check_spec(spec)?;
        let tx = self.conn.transaction()?;
        let n = tx.execute(
            "UPDATE charts SET spec = ?1, updated_at = ?2 WHERE id = ?3",
            params![json, stamp(now), id],
        )?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        let card = load_card(&tx, id)?;
        tx.commit()?;
        Ok(card)
    }

    /// # Errors
    /// `NotFound`, если карточки нет.
    pub fn chart_delete(&mut self, id: i64) -> Result<(), StorageError> {
        let n = self
            .conn
            .execute("DELETE FROM charts WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Сохраняет положение и размеры карточек дашборда за один раз (после перетаскивания).
    ///
    /// # Errors
    /// `Invalid`, если карточка выходит за 12 колонок, повторяется или чужая.
    pub fn charts_layout_set(
        &mut self,
        dashboard_id: i64,
        placements: &[CardPlacement],
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let mut seen = BTreeSet::new();
        for p in placements {
            check_size(p.w, p.h)?;
            if u16::from(p.x) + u16::from(p.w) > u16::from(GRID_COLUMNS) || !seen.insert(p.id) {
                return Err(StorageError::Invalid("dashboard.layout"));
            }
        }
        let tx = self.conn.transaction()?;
        let at = stamp(now);
        {
            let mut update = tx.prepare_cached(
                "UPDATE charts SET x = ?1, y = ?2, w = ?3, h = ?4, updated_at = ?5
                 WHERE id = ?6 AND dashboard_id = ?7",
            )?;
            for p in placements {
                let n = update.execute(params![p.x, p.y, p.w, p.h, at, p.id, dashboard_id])?;
                if n == 0 {
                    return Err(StorageError::Invalid("dashboard.layout"));
                }
            }
        }
        tx.commit()?;
        Ok(())
    }
}

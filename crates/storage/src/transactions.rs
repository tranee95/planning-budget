//! Траты: создание, правка, статус, мягкое удаление и восстановление.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::{CategoryId, Money, TagId, TxId, TxStatus, YearMonth};
use rusqlite::{Row, params};

use crate::records::{
    RecordSource, parse_date, parse_month, patched_period, period, validate_amount, validate_title,
};
use crate::{Db, StorageError, stamp};

const SELECT_TX: &str = "SELECT id, month, date, category_id, title, amount, status, comment, \
                         source, sort_key FROM";

/// Трата со всеми полями, которые видит пользователь.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TransactionRecord {
    pub id: TxId,
    pub month: YearMonth,
    pub date: Option<NaiveDate>,
    pub category_id: CategoryId,
    pub title: String,
    pub amount: Money,
    pub status: TxStatus,
    pub comment: Option<String>,
    pub source: RecordSource,
    pub sort_key: i64,
    pub tags: Vec<TagId>,
}

/// Поля новой траты. Если задана `date`, её месяц должен совпадать с `month`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NewTransaction {
    pub month: YearMonth,
    pub date: Option<NaiveDate>,
    pub category_id: CategoryId,
    pub title: String,
    pub amount: Money,
    pub status: TxStatus,
    pub comment: Option<String>,
    pub source: RecordSource,
}

/// Правка траты; `None` — не менять, `Some(None)` — очистить.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct TransactionPatch {
    pub month: Option<YearMonth>,
    pub date: Option<Option<NaiveDate>>,
    pub category_id: Option<CategoryId>,
    pub title: Option<String>,
    pub amount: Option<Money>,
    pub status: Option<TxStatus>,
    pub comment: Option<Option<String>>,
}

/// Результат правки: состояние до и после (для отмены в UI).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TransactionUpdate {
    pub previous: TransactionRecord,
    pub current: TransactionRecord,
}

pub(crate) fn status_str(status: TxStatus) -> &'static str {
    match status {
        TxStatus::Paid => "paid",
        TxStatus::Debt => "debt",
        TxStatus::Unplanned => "unplanned",
        TxStatus::Planned => "planned",
    }
}

pub(crate) fn status_from_str(s: &str) -> Result<TxStatus, StorageError> {
    match s {
        "paid" => Ok(TxStatus::Paid),
        "debt" => Ok(TxStatus::Debt),
        "unplanned" => Ok(TxStatus::Unplanned),
        "planned" => Ok(TxStatus::Planned),
        _ => Err(StorageError::Corrupt),
    }
}

pub(crate) fn map_tx(r: &Row<'_>) -> Result<TransactionRecord, StorageError> {
    let month: String = r.get("month")?;
    let date: Option<String> = r.get("date")?;
    let status: String = r.get("status")?;
    let source: String = r.get("source")?;
    Ok(TransactionRecord {
        id: TxId(r.get("id")?),
        month: parse_month(&month)?,
        date: date.as_deref().map(parse_date).transpose()?,
        category_id: CategoryId(r.get("category_id")?),
        title: r.get("title")?,
        amount: Money::from_kopecks(r.get("amount")?),
        status: status_from_str(&status)?,
        comment: r.get("comment")?,
        source: RecordSource::parse(&source)?,
        sort_key: r.get("sort_key")?,
        tags: Vec::new(),
    })
}

impl Db {
    fn tx_tags(
        &self,
        sql: &str,
        param: impl rusqlite::ToSql,
    ) -> Result<BTreeMap<i64, Vec<TagId>>, StorageError> {
        let mut stmt = self.conn.prepare_cached(sql)?;
        let mut rows = stmt.query([param])?;
        let mut out: BTreeMap<i64, Vec<TagId>> = BTreeMap::new();
        while let Some(row) = rows.next()? {
            out.entry(row.get(0)?).or_default().push(TagId(row.get(1)?));
        }
        Ok(out)
    }

    /// Траты месяца без мягко удалённых: по категориям, внутри — по `sort_key`.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn transactions_of_month(
        &self,
        month: YearMonth,
    ) -> Result<Vec<TransactionRecord>, StorageError> {
        let month = month.to_string();
        let mut tags = self.tx_tags(
            "SELECT tt.transaction_id, tt.tag_id FROM transaction_tags tt
             JOIN v_transactions t ON t.id = tt.transaction_id WHERE t.month = ?1
             ORDER BY tt.tag_id",
            &month,
        )?;
        let mut stmt = self.conn.prepare_cached(&format!(
            "{SELECT_TX} v_transactions WHERE month = ?1 ORDER BY category_id, sort_key, id"
        ))?;
        let mut rows = stmt.query([&month])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let mut record = map_tx(row)?;
            record.tags = tags.remove(&record.id.0).unwrap_or_default();
            out.push(record);
        }
        Ok(out)
    }

    /// Трата по id, в том числе мягко удалённая (нужна для ответа на `delete`/`restore`).
    ///
    /// # Errors
    /// [`StorageError::NotFound`], если записи нет.
    pub fn transaction(&self, id: TxId) -> Result<TransactionRecord, StorageError> {
        let mut tags = self.tx_tags(
            "SELECT transaction_id, tag_id FROM transaction_tags WHERE transaction_id = ?1 ORDER BY tag_id",
            id.0,
        )?;
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{SELECT_TX} transactions WHERE id = ?1"))?;
        let mut rows = stmt.query([id.0])?;
        let Some(row) = rows.next()? else {
            return Err(StorageError::NotFound);
        };
        let mut record = map_tx(row)?;
        record.tags = tags.remove(&id.0).unwrap_or_default();
        Ok(record)
    }

    /// Создаёт трату в конце блока своей категории.
    ///
    /// # Errors
    /// `Invalid` (сумма ≤ 0, пустое название, дата не из `month`, архивная категория),
    /// `NotFound` (нет категории).
    pub fn transaction_create(
        &mut self,
        input: &NewTransaction,
        now: DateTime<Utc>,
    ) -> Result<TransactionRecord, StorageError> {
        validate_amount(input.amount)?;
        let title = validate_title(&input.title)?;
        let (month, date) = period(input.month, input.date)?;
        self.require_active_category(input.category_id)?;
        let stamp = stamp(now);
        let tx = self.conn.transaction()?;
        let sort_key: i64 = tx.query_row(
            "SELECT COALESCE(max(sort_key) + 1, 0) FROM transactions
             WHERE month = ?1 AND category_id = ?2 AND deleted_at IS NULL",
            params![month.to_string(), input.category_id.0],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO transactions
                 (month, date, category_id, title, amount, status, comment, source, sort_key,
                  created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                month.to_string(),
                date.map(|d| d.to_string()),
                input.category_id.0,
                title,
                input.amount.kopecks(),
                status_str(input.status),
                input.comment,
                input.source.as_str(),
                sort_key,
                stamp
            ],
        )?;
        let id = TxId(tx.last_insert_rowid());
        tx.commit()?;
        self.transaction(id)
    }

    /// Правит трату и возвращает состояние до и после.
    ///
    /// # Errors
    /// `NotFound` (в том числе для мягко удалённой записи), `Invalid`.
    pub fn transaction_update(
        &mut self,
        id: TxId,
        patch: &TransactionPatch,
        now: DateTime<Utc>,
    ) -> Result<TransactionUpdate, StorageError> {
        let previous = self.transaction(id)?;
        self.ensure_live("transactions", id.0)?;
        let amount = patch.amount.unwrap_or(previous.amount);
        validate_amount(amount)?;
        let title = match &patch.title {
            Some(title) => validate_title(title)?.to_owned(),
            None => previous.title.clone(),
        };
        let category_id = patch.category_id.unwrap_or(previous.category_id);
        if category_id != previous.category_id {
            self.require_active_category(category_id)?;
        }
        let (month, date) =
            patched_period((previous.month, previous.date), patch.month, patch.date)?;
        let comment = patch
            .comment
            .clone()
            .unwrap_or_else(|| previous.comment.clone());
        let status = patch.status.unwrap_or(previous.status);
        // Перенос в другой блок ставит запись в его конец.
        let moved = month != previous.month || category_id != previous.category_id;
        let tx = self.conn.transaction()?;
        let sort_key: i64 = if moved {
            tx.query_row(
                "SELECT COALESCE(max(sort_key) + 1, 0) FROM transactions
                 WHERE month = ?1 AND category_id = ?2 AND deleted_at IS NULL",
                params![month.to_string(), category_id.0],
                |r| r.get(0),
            )?
        } else {
            previous.sort_key
        };
        tx.execute(
            "UPDATE transactions SET month = ?2, date = ?3, category_id = ?4, title = ?5,
                 amount = ?6, status = ?7, comment = ?8, sort_key = ?9, updated_at = ?10,
                 planned_amount = CASE WHEN ?11 THEN NULL ELSE planned_amount END
             WHERE id = ?1",
            params![
                id.0,
                month.to_string(),
                date.map(|d| d.to_string()),
                category_id.0,
                title,
                amount.kopecks(),
                status_str(status),
                comment,
                sort_key,
                stamp(now),
                month != previous.month
            ],
        )?;
        tx.commit()?;
        Ok(TransactionUpdate {
            previous,
            current: self.transaction(id)?,
        })
    }

    /// Мягко удаляет трату: она пропадает из списков, сводок и поиска, но восстановима.
    ///
    /// # Errors
    /// `NotFound`, если траты нет или она уже удалена.
    pub fn transaction_delete(&mut self, id: TxId, now: DateTime<Utc>) -> Result<(), StorageError> {
        self.soft_delete("transactions", id.0, now)
    }

    /// Возвращает мягко удалённую трату.
    ///
    /// # Errors
    /// `NotFound`, если траты нет или она не удалена; `Conflict`, если на её место
    /// успела встать запись с тем же `fingerprint`.
    pub fn transaction_restore(
        &mut self,
        id: TxId,
        now: DateTime<Utc>,
    ) -> Result<TransactionRecord, StorageError> {
        self.restore("transactions", id.0, now)?;
        self.transaction(id)
    }

    /// Заменяет набор тегов траты.
    ///
    /// # Errors
    /// `NotFound`, если нет траты или одного из тегов.
    pub fn transaction_tags_set(&mut self, id: TxId, tags: &[TagId]) -> Result<(), StorageError> {
        self.ensure_live("transactions", id.0)?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM transaction_tags WHERE transaction_id = ?1",
            [id.0],
        )?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT OR IGNORE INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
            )?;
            for tag in tags {
                insert.execute(params![id.0, tag.0]).map_err(|e| {
                    let e = StorageError::from(e);
                    if e.is_constraint() {
                        StorageError::NotFound
                    } else {
                        e
                    }
                })?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Запись существует и не удалена. `table` — только константа вызывающего кода.
    pub(crate) fn ensure_live(&self, table: &'static str, id: i64) -> Result<(), StorageError> {
        let live: bool = self.conn.query_row(
            &format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE id = ?1 AND deleted_at IS NULL)"),
            [id],
            |r| r.get(0),
        )?;
        if live {
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    pub(crate) fn soft_delete(
        &mut self,
        table: &'static str,
        id: i64,
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let stamp = stamp(now);
        let changed = self.conn.execute(
            &format!("UPDATE {table} SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL"),
            params![id, stamp],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub(crate) fn restore(
        &mut self,
        table: &'static str,
        id: i64,
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let changed = self
            .conn
            .execute(
                &format!("UPDATE {table} SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NOT NULL"),
                params![id, stamp(now)],
            )
            .map_err(|e| {
                let e = StorageError::from(e);
                if e.is_constraint() { StorageError::Conflict("record.duplicate") } else { e }
            })?;
        if changed == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

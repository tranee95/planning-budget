//! Доходы: создание, правка, статус, мягкое удаление и восстановление.

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::{IncomeId, IncomeStatus, Money, YearMonth};
use rusqlite::{Row, params};

use crate::records::{
    RecordSource, parse_date, parse_month, patched_period, period, validate_amount, validate_title,
};
use crate::{Db, StorageError, stamp};

const SELECT_INCOME: &str =
    "SELECT id, month, date, source_name, amount, status, comment, source FROM";

/// Доход со всеми полями, которые видит пользователь.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IncomeRecord {
    pub id: IncomeId,
    pub month: YearMonth,
    pub date: Option<NaiveDate>,
    pub source_name: String,
    pub amount: Money,
    pub status: IncomeStatus,
    pub comment: Option<String>,
    pub source: RecordSource,
}

/// Поля нового дохода. Если задана `date`, её месяц должен совпадать с `month`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NewIncome {
    pub month: YearMonth,
    pub date: Option<NaiveDate>,
    pub source_name: String,
    pub amount: Money,
    pub status: IncomeStatus,
    pub comment: Option<String>,
    pub source: RecordSource,
}

/// Правка дохода; `None` — не менять, `Some(None)` — очистить.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct IncomePatch {
    pub month: Option<YearMonth>,
    pub date: Option<Option<NaiveDate>>,
    pub source_name: Option<String>,
    pub amount: Option<Money>,
    pub status: Option<IncomeStatus>,
    pub comment: Option<Option<String>>,
}

/// Результат правки: состояние до и после (для отмены в UI).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IncomeUpdate {
    pub previous: IncomeRecord,
    pub current: IncomeRecord,
}

pub(crate) fn status_str(status: IncomeStatus) -> &'static str {
    match status {
        IncomeStatus::Received => "received",
        IncomeStatus::Expected => "expected",
    }
}

pub(crate) fn status_from_str(s: &str) -> Result<IncomeStatus, StorageError> {
    match s {
        "received" => Ok(IncomeStatus::Received),
        "expected" => Ok(IncomeStatus::Expected),
        _ => Err(StorageError::Corrupt),
    }
}

pub(crate) fn map_income(r: &Row<'_>) -> Result<IncomeRecord, StorageError> {
    let month: String = r.get("month")?;
    let date: Option<String> = r.get("date")?;
    let status: String = r.get("status")?;
    let source: String = r.get("source")?;
    Ok(IncomeRecord {
        id: IncomeId(r.get("id")?),
        month: parse_month(&month)?,
        date: date.as_deref().map(parse_date).transpose()?,
        source_name: r.get("source_name")?,
        amount: Money::from_kopecks(r.get("amount")?),
        status: status_from_str(&status)?,
        comment: r.get("comment")?,
        source: RecordSource::parse(&source)?,
    })
}

impl Db {
    /// Доходы месяца без мягко удалённых, по порядку добавления.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn incomes_of_month(&self, month: YearMonth) -> Result<Vec<IncomeRecord>, StorageError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "{SELECT_INCOME} v_incomes WHERE month = ?1 ORDER BY id"
        ))?;
        let mut rows = stmt.query([month.to_string()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_income(row)?);
        }
        Ok(out)
    }

    /// Доход по id, в том числе мягко удалённый.
    ///
    /// # Errors
    /// [`StorageError::NotFound`], если записи нет.
    pub fn income(&self, id: IncomeId) -> Result<IncomeRecord, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{SELECT_INCOME} incomes WHERE id = ?1"))?;
        let mut rows = stmt.query([id.0])?;
        match rows.next()? {
            Some(row) => map_income(row),
            None => Err(StorageError::NotFound),
        }
    }

    /// Создаёт доход.
    ///
    /// # Errors
    /// `Invalid` (сумма ≤ 0, пустое название источника, дата не из `month`).
    pub fn income_create(
        &mut self,
        input: &NewIncome,
        now: DateTime<Utc>,
    ) -> Result<IncomeRecord, StorageError> {
        validate_amount(input.amount)?;
        let source_name = validate_title(&input.source_name)?;
        let (month, date) = period(input.month, input.date)?;
        let stamp = stamp(now);
        self.conn.execute(
            "INSERT INTO incomes
                 (month, date, source_name, amount, status, comment, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                month.to_string(),
                date.map(|d| d.to_string()),
                source_name,
                input.amount.kopecks(),
                status_str(input.status),
                input.comment,
                input.source.as_str(),
                stamp
            ],
        )?;
        self.income(IncomeId(self.conn.last_insert_rowid()))
    }

    /// Правит доход и возвращает состояние до и после.
    ///
    /// # Errors
    /// `NotFound` (в том числе для мягко удалённой записи), `Invalid`.
    pub fn income_update(
        &mut self,
        id: IncomeId,
        patch: &IncomePatch,
        now: DateTime<Utc>,
    ) -> Result<IncomeUpdate, StorageError> {
        let previous = self.income(id)?;
        self.ensure_live("incomes", id.0)?;
        let amount = patch.amount.unwrap_or(previous.amount);
        validate_amount(amount)?;
        let source_name = match &patch.source_name {
            Some(name) => validate_title(name)?.to_owned(),
            None => previous.source_name.clone(),
        };
        let (month, date) =
            patched_period((previous.month, previous.date), patch.month, patch.date)?;
        let comment = patch
            .comment
            .clone()
            .unwrap_or_else(|| previous.comment.clone());
        let status = patch.status.unwrap_or(previous.status);
        self.conn.execute(
            "UPDATE incomes SET month = ?2, date = ?3, source_name = ?4, amount = ?5,
                 status = ?6, comment = ?7, updated_at = ?8
             WHERE id = ?1",
            params![
                id.0,
                month.to_string(),
                date.map(|d| d.to_string()),
                source_name,
                amount.kopecks(),
                status_str(status),
                comment,
                stamp(now)
            ],
        )?;
        Ok(IncomeUpdate {
            previous,
            current: self.income(id)?,
        })
    }

    /// Мягко удаляет доход.
    ///
    /// # Errors
    /// `NotFound`, если дохода нет или он уже удалён.
    pub fn income_delete(&mut self, id: IncomeId, now: DateTime<Utc>) -> Result<(), StorageError> {
        self.soft_delete("incomes", id.0, now)
    }

    /// Возвращает мягко удалённый доход.
    ///
    /// # Errors
    /// `NotFound`, если дохода нет или он не удалён; `Conflict` при совпадении `fingerprint`.
    pub fn income_restore(
        &mut self,
        id: IncomeId,
        now: DateTime<Utc>,
    ) -> Result<IncomeRecord, StorageError> {
        self.restore("incomes", id.0, now)?;
        self.income(id)
    }
}

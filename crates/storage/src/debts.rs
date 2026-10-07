//! Долги (займы) и графики погашений: создание, правка, оплата строк, мягкое удаление
//!.

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::calc::validate_schedule;
use planning_budget_core::{CategoryId, DebtId, DebtPaymentStatus, Money, TxId, YearMonth};
use rusqlite::{Row, params};

use crate::dataset::{payment_status_from_str, payment_status_str};
use crate::records::{month_of, parse_date, parse_month, validate_amount};
use crate::{Db, StorageError, stamp};

const SELECT_DEBT: &str = "SELECT id, lender, amount, taken_month, taken_date, category_id, \
                           transaction_id, comment, closed_at FROM";

/// Строка графика погашения.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DebtPaymentRecord {
    pub id: i64,
    pub month: YearMonth,
    pub amount: Money,
    pub status: DebtPaymentStatus,
    pub paid_date: Option<NaiveDate>,
}

/// Долг со всем графиком.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DebtRecord {
    pub id: DebtId,
    pub lender: String,
    pub amount: Money,
    pub taken_month: YearMonth,
    pub taken_date: Option<NaiveDate>,
    pub category_id: Option<CategoryId>,
    pub transaction_id: Option<TxId>,
    pub comment: Option<String>,
    /// Все платежи оплачены.
    pub closed: bool,
    pub payments: Vec<DebtPaymentRecord>,
}

/// Поля нового долга. Сумма графика должна равняться сумме долга.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NewDebt {
    pub lender: String,
    pub amount: Money,
    pub taken_month: YearMonth,
    pub taken_date: Option<NaiveDate>,
    pub category_id: Option<CategoryId>,
    pub transaction_id: Option<TxId>,
    pub comment: Option<String>,
    pub schedule: Vec<(YearMonth, Money)>,
}

/// Правка долга; `None` — не менять, `Some(None)` — очистить. `schedule` заменяет только неоплаченные
/// строки: оплаченные не меняются, и вместе с ними график должен давать сумму долга.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct DebtPatch {
    pub lender: Option<String>,
    pub amount: Option<Money>,
    pub category_id: Option<Option<CategoryId>>,
    pub comment: Option<Option<String>>,
    pub schedule: Option<Vec<(YearMonth, Money)>>,
}

fn validate_lender(lender: &str) -> Result<&str, StorageError> {
    let lender = lender.trim();
    if lender.is_empty() {
        Err(StorageError::Invalid("debt.lender_empty"))
    } else {
        Ok(lender)
    }
}

fn schedule_error<E>(_: E) -> StorageError {
    StorageError::Invalid("debt.schedule")
}

fn map_debt(row: &Row<'_>) -> Result<DebtRecord, StorageError> {
    let taken_month: String = row.get(3)?;
    let taken_date: Option<String> = row.get(4)?;
    Ok(DebtRecord {
        id: DebtId(row.get(0)?),
        lender: row.get(1)?,
        amount: Money::from_kopecks(row.get(2)?),
        taken_month: parse_month(&taken_month)?,
        taken_date: taken_date.as_deref().map(parse_date).transpose()?,
        category_id: row.get::<_, Option<i64>>(5)?.map(CategoryId),
        transaction_id: row.get::<_, Option<i64>>(6)?.map(TxId),
        comment: row.get(7)?,
        closed: row.get::<_, Option<String>>(8)?.is_some(),
        payments: Vec::new(),
    })
}

impl Db {
    /// Долг с графиком.
    ///
    /// # Errors
    /// `NotFound`, если долга нет или он удалён.
    pub fn debt(&self, id: DebtId) -> Result<DebtRecord, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{SELECT_DEBT} v_debts WHERE id = ?1"))?;
        let mut rows = stmt.query([id.0])?;
        let mut debt = match rows.next()? {
            Some(row) => map_debt(row)?,
            None => return Err(StorageError::NotFound),
        };
        debt.payments = self.debt_payments_of(id)?;
        Ok(debt)
    }

    /// Долги: открытые первыми; закрытые — только по запросу.
    ///
    /// # Errors
    /// Ошибка SQLite или повреждённая строка.
    pub fn debts(&self, include_closed: bool) -> Result<Vec<DebtRecord>, StorageError> {
        let filter = if include_closed {
            ""
        } else {
            "WHERE closed_at IS NULL"
        };
        let mut stmt = self.conn.prepare_cached(&format!(
            "{SELECT_DEBT} v_debts {filter} ORDER BY closed_at IS NOT NULL, taken_month, id"
        ))?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_debt(row)?);
        }
        for debt in &mut out {
            debt.payments = self.debt_payments_of(debt.id)?;
        }
        Ok(out)
    }

    fn debt_payments_of(&self, id: DebtId) -> Result<Vec<DebtPaymentRecord>, StorageError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, month, amount, status, paid_date FROM debt_payments
             WHERE debt_id = ?1 ORDER BY month",
        )?;
        let mut rows = stmt.query([id.0])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let month: String = row.get(1)?;
            let status: String = row.get(3)?;
            let paid_date: Option<String> = row.get(4)?;
            out.push(DebtPaymentRecord {
                id: row.get(0)?,
                month: parse_month(&month)?,
                amount: Money::from_kopecks(row.get(2)?),
                status: payment_status_from_str(&status)?,
                paid_date: paid_date.as_deref().map(parse_date).transpose()?,
            });
        }
        Ok(out)
    }

    /// Создаёт долг и плановые строки графика.
    ///
    /// # Errors
    /// `Invalid`: пустое имя, сумма вне допустимой, дата не из месяца займа, график не равен сумме
    /// долга (`debt.schedule`); `NotFound`: категории или траты нет.
    pub fn debt_create(
        &mut self,
        input: &NewDebt,
        now: DateTime<Utc>,
    ) -> Result<DebtRecord, StorageError> {
        let lender = validate_lender(&input.lender)?;
        validate_amount(input.amount)?;
        if let Some(date) = input.taken_date
            && month_of(date)? != input.taken_month
        {
            return Err(StorageError::Invalid("debt.date_month_mismatch"));
        }
        if let Some(category) = input.category_id {
            self.category(category)?;
        }
        if let Some(tx) = input.transaction_id {
            self.ensure_live("transactions", tx.0)?;
            let linked: bool = self.conn.query_row(
                "SELECT EXISTS (SELECT 1 FROM v_debts WHERE transaction_id = ?1)",
                [tx.0],
                |r| r.get(0),
            )?;
            if linked {
                return Err(StorageError::Conflict("debt.transaction_linked"));
            }
        }
        validate_schedule(input.amount, input.taken_month, &input.schedule)
            .map_err(schedule_error)?;

        let stamp = stamp(now);
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO debts (lender, amount, taken_month, taken_date, category_id, transaction_id,
                                comment, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                lender,
                input.amount.kopecks(),
                input.taken_month.to_string(),
                input.taken_date.map(|d| d.to_string()),
                input.category_id.map(|c| c.0),
                input.transaction_id.map(|t| t.0),
                input.comment,
                stamp
            ],
        )?;
        let id = tx.last_insert_rowid();
        for (month, amount) in &input.schedule {
            tx.execute(
                "INSERT INTO debt_payments (debt_id, month, amount, status) VALUES (?1, ?2, ?3, 'planned')",
                params![id, month.to_string(), amount.kopecks()],
            )?;
        }
        tx.commit()?;
        self.debt(DebtId(id))
    }

    /// Создаёт долг из траты: сумма, статья, месяц и дата берутся из неё.
    ///
    /// # Errors
    /// `NotFound`, если траты нет; остальное как у [`Db::debt_create`].
    pub fn debt_create_from_transaction(
        &mut self,
        tx_id: TxId,
        lender: &str,
        schedule: Vec<(YearMonth, Money)>,
        now: DateTime<Utc>,
    ) -> Result<DebtRecord, StorageError> {
        let t = self.transaction(tx_id)?;
        self.debt_create(
            &NewDebt {
                lender: lender.to_owned(),
                amount: t.amount,
                taken_month: t.month,
                taken_date: t.date,
                category_id: Some(t.category_id),
                transaction_id: Some(tx_id),
                comment: None,
                schedule,
            },
            now,
        )
    }

    /// Правит долг. Оплаченные строки графика не меняются.
    ///
    /// # Errors
    /// `NotFound`; `Invalid` (`debt.schedule`), если новая сумма требует нового графика или график с
    /// оплаченными строками не равен сумме долга.
    pub fn debt_update(
        &mut self,
        id: DebtId,
        patch: &DebtPatch,
        now: DateTime<Utc>,
    ) -> Result<DebtRecord, StorageError> {
        let current = self.debt(id)?;
        let lender = match &patch.lender {
            Some(l) => validate_lender(l)?.to_owned(),
            None => current.lender.clone(),
        };
        let amount = patch.amount.unwrap_or(current.amount);
        validate_amount(amount)?;
        let category = match patch.category_id {
            Some(c) => c,
            None => current.category_id,
        };
        if let Some(category) = category {
            self.category(category)?;
        }
        let comment = match &patch.comment {
            Some(c) => c.clone(),
            None => current.comment.clone(),
        };

        // Новый график заменяет только неоплаченные строки; оплаченные остаются как были.
        let paid: Vec<(YearMonth, Money)> = current
            .payments
            .iter()
            .filter(|p| p.status == DebtPaymentStatus::Paid)
            .map(|p| (p.month, p.amount))
            .collect();
        let replace_schedule = patch.schedule.is_some() || amount != current.amount;
        let unpaid: Vec<(YearMonth, Money)> = match &patch.schedule {
            Some(rows) => rows.clone(),
            None if amount != current.amount => {
                return Err(StorageError::Invalid("debt.schedule"));
            }
            None => Vec::new(),
        };
        if replace_schedule {
            let mut full = paid.clone();
            full.extend(unpaid.iter().copied());
            validate_schedule(amount, current.taken_month, &full).map_err(schedule_error)?;
        }

        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE debts SET lender = ?2, amount = ?3, category_id = ?4, comment = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                id.0,
                lender,
                amount.kopecks(),
                category.map(|c| c.0),
                comment,
                stamp(now)
            ],
        )?;
        if replace_schedule {
            tx.execute(
                "DELETE FROM debt_payments WHERE debt_id = ?1 AND status = 'planned'",
                [id.0],
            )?;
            for (month, value) in &unpaid {
                tx.execute(
                    "INSERT INTO debt_payments (debt_id, month, amount, status) VALUES (?1, ?2, ?3, 'planned')",
                    params![id.0, month.to_string(), value.kopecks()],
                )?;
            }
        }
        refresh_closed(&tx, id, now)?;
        tx.commit()?;
        self.debt(id)
    }

    /// Отмечает строку графика оплаченной или возвращает её в план. Долг закрывается сам, когда
    /// оплачены все строки.
    ///
    /// # Errors
    /// `NotFound`, если строки нет или её долг удалён; `Invalid`, если дата оплаты не вида `YYYY-MM-DD`
    /// невозможна (проверяется типом).
    pub fn debt_payment_set_status(
        &mut self,
        payment_id: i64,
        status: DebtPaymentStatus,
        paid_date: Option<NaiveDate>,
        now: DateTime<Utc>,
    ) -> Result<DebtRecord, StorageError> {
        let debt_id: i64 = {
            let mut stmt = self.conn.prepare_cached(
                "SELECT p.debt_id FROM debt_payments p JOIN v_debts d ON d.id = p.debt_id
                 WHERE p.id = ?1",
            )?;
            let mut rows = stmt.query([payment_id])?;
            match rows.next()? {
                Some(row) => row.get(0)?,
                None => return Err(StorageError::NotFound),
            }
        };
        let date = match status {
            DebtPaymentStatus::Paid => paid_date.map(|d| d.to_string()),
            DebtPaymentStatus::Planned => None,
        };
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE debt_payments SET status = ?2, paid_date = ?3 WHERE id = ?1",
            params![payment_id, payment_status_str(status), date],
        )?;
        refresh_closed(&tx, DebtId(debt_id), now)?;
        tx.commit()?;
        self.debt(DebtId(debt_id))
    }

    /// Мягкое удаление долга (для отмены в UI).
    ///
    /// # Errors
    /// `NotFound`.
    pub fn debt_delete(&mut self, id: DebtId, now: DateTime<Utc>) -> Result<(), StorageError> {
        self.soft_delete("debts", id.0, now)
    }

    /// Возвращает удалённый долг.
    ///
    /// # Errors
    /// `NotFound`.
    pub fn debt_restore(
        &mut self,
        id: DebtId,
        now: DateTime<Utc>,
    ) -> Result<DebtRecord, StorageError> {
        self.restore("debts", id.0, now)?;
        self.debt(id)
    }
}

/// `closed_at` ставится, когда оплачена вся сумма долга, и снимается, если оплату отменили.
fn refresh_closed(
    conn: &rusqlite::Connection,
    id: DebtId,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE debts SET closed_at = CASE
             WHEN (SELECT COALESCE(SUM(amount), 0) FROM debt_payments
                   WHERE debt_id = debts.id AND status = 'paid') = debts.amount
             THEN COALESCE(closed_at, ?2) ELSE NULL END
         WHERE id = ?1",
        params![id.0, stamp(now)],
    )?;
    Ok(())
}

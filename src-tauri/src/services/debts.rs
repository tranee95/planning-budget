//! Долги: сервис над хранилищем и `core::calc`.

use chrono::{DateTime, Utc};
use planning_budget_core::calc::{
    Ledger, debt_progress, equal_parts_schedule, single_payment_schedule,
};
use planning_budget_core::{CategoryId, DebtId, DebtPayment, Money, TxId, YearMonth};
use planning_budget_storage::{Db, DebtPatch, DebtRecord, NewDebt, StorageError};

use super::{parse_date, parse_month};
use crate::AppError;
use crate::dto::{
    DebtDto, DebtInput, DebtPatchDto, DebtPaymentDto, DebtPaymentStatusDto, DebtScheduleKindDto,
    DebtsOverviewDto, SchedulePaymentDto, SchedulePreviewDto,
};

/// График на входе → строки `(месяц, сумма)`.
fn schedule(rows: &[SchedulePaymentDto]) -> Result<Vec<(YearMonth, Money)>, AppError> {
    rows.iter()
        .map(|r| {
            Ok((
                parse_month(&r.month, "month")?,
                Money::from_kopecks(r.amount),
            ))
        })
        .collect()
}

fn schedule_error<E>(_: E) -> AppError {
    StorageError::Invalid("debt.schedule").into()
}

fn payment_dto(p: &planning_budget_storage::DebtPaymentRecord) -> DebtPaymentDto {
    DebtPaymentDto {
        id: p.id,
        month: p.month.to_string(),
        amount: p.amount.kopecks(),
        status: p.status.into(),
        paid_date: p.paid_date.map(|d| d.to_string()),
    }
}

fn to_dto(rec: &DebtRecord) -> Result<DebtDto, AppError> {
    let rows: Vec<DebtPayment> = rec
        .payments
        .iter()
        .map(|p| DebtPayment {
            debt_id: rec.id,
            month: p.month,
            amount: p.amount,
            status: p.status,
        })
        .collect();
    let state = debt_progress(rec.amount, &rows)?;
    let paid = rec
        .amount
        .checked_sub(state.remaining)
        .map_err(planning_budget_core::CoreError::from)?;
    let next_payment = state.next_payment.and_then(|n| {
        rec.payments
            .iter()
            .find(|p| p.month == n.month)
            .map(payment_dto)
    });
    Ok(DebtDto {
        id: rec.id.0,
        lender: rec.lender.clone(),
        amount: rec.amount.kopecks(),
        taken_month: rec.taken_month.to_string(),
        taken_date: rec.taken_date.map(|d| d.to_string()),
        category_id: rec.category_id.map(|c| c.0),
        transaction_id: rec.transaction_id.map(|t| t.0),
        comment: rec.comment.clone(),
        closed: rec.closed,
        remaining: state.remaining.kopecks(),
        paid_bp: paid.ratio_bp(rec.amount).unwrap_or(0),
        next_payment,
        payments: rec.payments.iter().map(payment_dto).collect(),
    })
}

/// Раздел «Долги» на месяц: список и итоги. Закрытые долги входят только по запросу.
pub fn overview(db: &Db, month: &str, include_closed: bool) -> Result<DebtsOverviewDto, AppError> {
    let month = parse_month(month, "month")?;
    let data = db.dataset(month, month)?;
    let summary = Ledger::new(&data)?.debts_summary(month)?;
    let debts = db
        .debts(include_closed)?
        .iter()
        .map(to_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DebtsOverviewDto {
        debts,
        open_count: summary.open_count,
        closed_count: summary.closed_count,
        remaining_total: summary.remaining_total.kopecks(),
        payments_planned: summary.payments_planned.kopecks(),
        payments_paid: summary.payments_paid.kopecks(),
    })
}

pub fn create(db: &mut Db, input: DebtInput, now: DateTime<Utc>) -> Result<DebtDto, AppError> {
    let record = db.debt_create(
        &NewDebt {
            lender: input.lender,
            amount: Money::from_kopecks(input.amount),
            taken_month: parse_month(&input.taken_month, "takenMonth")?,
            taken_date: input
                .taken_date
                .as_deref()
                .map(|d| parse_date(d, "takenDate"))
                .transpose()?,
            category_id: input.category_id.map(CategoryId),
            transaction_id: None,
            comment: input.comment,
            schedule: schedule(&input.schedule)?,
        },
        now,
    )?;
    to_dto(&record)
}

/// Долг из траты со статусом «Долг»: сумма, статья и месяц берутся из траты.
pub fn create_from_transaction(
    db: &mut Db,
    tx_id: TxId,
    lender: &str,
    rows: &[SchedulePaymentDto],
    now: DateTime<Utc>,
) -> Result<DebtDto, AppError> {
    let record = db.debt_create_from_transaction(tx_id, lender, schedule(rows)?, now)?;
    to_dto(&record)
}

pub fn update(
    db: &mut Db,
    id: DebtId,
    patch: DebtPatchDto,
    now: DateTime<Utc>,
) -> Result<DebtDto, AppError> {
    let record = db.debt_update(
        id,
        &DebtPatch {
            lender: patch.lender,
            amount: patch.amount.map(Money::from_kopecks),
            category_id: patch.category_id.map(|c| c.map(CategoryId)),
            comment: patch.comment,
            schedule: patch.schedule.as_deref().map(schedule).transpose()?,
        },
        now,
    )?;
    to_dto(&record)
}

pub fn set_payment_status(
    db: &mut Db,
    payment_id: i64,
    status: DebtPaymentStatusDto,
    paid_date: Option<&str>,
    now: DateTime<Utc>,
) -> Result<DebtDto, AppError> {
    let date = paid_date.map(|d| parse_date(d, "paidDate")).transpose()?;
    let record = db.debt_payment_set_status(payment_id, status.into(), date, now)?;
    to_dto(&record)
}

/// Мягко удаляет долг и возвращает его прежнее состояние (для отмены в UI).
pub fn delete(db: &mut Db, id: DebtId, now: DateTime<Utc>) -> Result<DebtDto, AppError> {
    let before = to_dto(&db.debt(id)?)?;
    db.debt_delete(id, now)?;
    Ok(before)
}

pub fn restore(db: &mut Db, id: DebtId, now: DateTime<Utc>) -> Result<DebtDto, AppError> {
    to_dto(&db.debt_restore(id, now)?)
}

/// Быстрый график: «равными частями на N месяцев» (со следующего за займом месяца) или
/// «одним платежом в месяце M». Остаток копеек уходит в последний платёж.
pub fn schedule_preview(
    amount: i64,
    taken_month: &str,
    kind: DebtScheduleKindDto,
) -> Result<SchedulePreviewDto, AppError> {
    let taken = parse_month(taken_month, "takenMonth")?;
    let amount = Money::from_kopecks(amount);
    let rows = match kind {
        DebtScheduleKindDto::EqualParts { months } => {
            let first = taken
                .succ()
                .ok_or_else(|| AppError::invalid_field("month_invalid", "takenMonth"))?;
            equal_parts_schedule(amount, first, months)
        }
        DebtScheduleKindDto::Single { month } => {
            single_payment_schedule(amount, taken, parse_month(&month, "month")?)
        }
    }
    .map_err(schedule_error)?;
    let total = Money::sum(rows.iter().map(|(_, amount)| *amount))
        .map_err(planning_budget_core::CoreError::from)?;
    Ok(SchedulePreviewDto {
        rows: rows
            .into_iter()
            .map(|(month, amount)| SchedulePaymentDto {
                month: month.to_string(),
                amount: amount.kopecks(),
            })
            .collect(),
        total: total.kopecks(),
    })
}

/// Месяцы, которых коснулось изменение долга: месяц займа и месяцы графика.
pub fn touched_months(dto: &DebtDto) -> Vec<String> {
    let mut months: Vec<String> = dto.payments.iter().map(|p| p.month.clone()).collect();
    months.push(dto.taken_month.clone());
    months.sort();
    months.dedup();
    months
}

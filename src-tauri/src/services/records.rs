//! Траты, доходы и теги.

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::{CategoryId, IncomeId, Money, TagId, TxId};
use planning_budget_storage::{
    Db, IncomePatch, NewIncome, NewTransaction, RecordSource, StorageError, TransactionPatch,
};

use super::{parse_date, parse_month};
use crate::AppError;
use crate::dto::{
    IncomeDto, IncomeInput, IncomePatchDto, IncomeUpdateResultDto, TagDto, TransactionDto,
    TransactionInput, TransactionPatchDto, TxStatusDto, TxUpdateResultDto,
};

fn tx_error(id: TxId) -> impl Fn(StorageError) -> AppError {
    move |e| AppError::from_storage(e, "transaction", id.0)
}

fn income_error(id: IncomeId) -> impl Fn(StorageError) -> AppError {
    move |e| AppError::from_storage(e, "income", id.0)
}

fn optional_date(value: Option<&String>, field: &str) -> Result<Option<NaiveDate>, AppError> {
    value.map(|d| parse_date(d, field)).transpose()
}

/// `Some(None)` очищает поле, `Some(Some(_))` задаёт, `None` оставляет как есть.
fn date_patch(
    value: Option<Option<String>>,
    field: &str,
) -> Result<Option<Option<NaiveDate>>, AppError> {
    value
        .map(|d| d.map(|d| parse_date(&d, field)).transpose())
        .transpose()
}

pub fn tx_list(db: &Db, month: &str) -> Result<Vec<TransactionDto>, AppError> {
    let month = parse_month(month, "month")?;
    Ok(db
        .transactions_of_month(month)?
        .iter()
        .map(TransactionDto::from)
        .collect())
}

pub fn tx_create(
    db: &mut Db,
    input: TransactionInput,
    now: DateTime<Utc>,
) -> Result<TransactionDto, AppError> {
    let created = db.transaction_create(
        &NewTransaction {
            month: parse_month(&input.month, "month")?,
            date: optional_date(input.date.as_ref(), "date")?,
            category_id: CategoryId(input.category_id),
            title: input.title,
            amount: Money::from_kopecks(input.amount),
            status: input.status.into(),
            comment: input.comment,
            source: RecordSource::Manual,
        },
        now,
    )?;
    Ok((&created).into())
}

pub fn tx_update(
    db: &mut Db,
    id: TxId,
    patch: TransactionPatchDto,
    now: DateTime<Utc>,
) -> Result<TxUpdateResultDto, AppError> {
    let result = db
        .transaction_update(
            id,
            &TransactionPatch {
                month: patch
                    .month
                    .as_deref()
                    .map(|m| parse_month(m, "month"))
                    .transpose()?,
                date: date_patch(patch.date, "date")?,
                category_id: patch.category_id.map(CategoryId),
                title: patch.title,
                amount: patch.amount.map(Money::from_kopecks),
                status: patch.status.map(Into::into),
                comment: patch.comment,
            },
            now,
        )
        .map_err(tx_error(id))?;
    Ok(TxUpdateResultDto {
        current: (&result.current).into(),
        previous: (&result.previous).into(),
    })
}

pub fn tx_set_status(
    db: &mut Db,
    id: TxId,
    status: TxStatusDto,
    now: DateTime<Utc>,
) -> Result<TxUpdateResultDto, AppError> {
    tx_update(
        db,
        id,
        TransactionPatchDto {
            status: Some(status),
            ..TransactionPatchDto::default()
        },
        now,
    )
}

/// Мягко удаляет трату и возвращает её прежнее состояние: по нему UI обновляет кэш месяца.
pub fn tx_delete(db: &mut Db, id: TxId, now: DateTime<Utc>) -> Result<TransactionDto, AppError> {
    let before = db.transaction(id).map_err(tx_error(id))?;
    db.transaction_delete(id, now).map_err(tx_error(id))?;
    Ok((&before).into())
}

pub fn tx_restore(db: &mut Db, id: TxId, now: DateTime<Utc>) -> Result<TransactionDto, AppError> {
    let restored = db.transaction_restore(id, now).map_err(tx_error(id))?;
    Ok((&restored).into())
}

pub fn tx_tags_set(db: &mut Db, id: TxId, tags: Vec<i64>) -> Result<TransactionDto, AppError> {
    let tags: Vec<TagId> = tags.into_iter().map(TagId).collect();
    db.transaction_tags_set(id, &tags).map_err(tx_error(id))?;
    let updated = db.transaction(id).map_err(tx_error(id))?;
    Ok((&updated).into())
}

pub fn incomes_list(db: &Db, month: &str) -> Result<Vec<IncomeDto>, AppError> {
    let month = parse_month(month, "month")?;
    Ok(db
        .incomes_of_month(month)?
        .iter()
        .map(IncomeDto::from)
        .collect())
}

pub fn income_create(
    db: &mut Db,
    input: IncomeInput,
    now: DateTime<Utc>,
) -> Result<IncomeDto, AppError> {
    let created = db.income_create(
        &NewIncome {
            month: parse_month(&input.month, "month")?,
            date: optional_date(input.date.as_ref(), "date")?,
            source_name: input.source_name,
            amount: Money::from_kopecks(input.amount),
            status: input.status.into(),
            comment: input.comment,
            source: RecordSource::Manual,
        },
        now,
    )?;
    Ok((&created).into())
}

pub fn income_update(
    db: &mut Db,
    id: IncomeId,
    patch: IncomePatchDto,
    now: DateTime<Utc>,
) -> Result<IncomeUpdateResultDto, AppError> {
    let result = db
        .income_update(
            id,
            &IncomePatch {
                month: patch
                    .month
                    .as_deref()
                    .map(|m| parse_month(m, "month"))
                    .transpose()?,
                date: date_patch(patch.date, "date")?,
                source_name: patch.source_name,
                amount: patch.amount.map(Money::from_kopecks),
                status: patch.status.map(Into::into),
                comment: patch.comment,
            },
            now,
        )
        .map_err(income_error(id))?;
    Ok(IncomeUpdateResultDto {
        current: (&result.current).into(),
        previous: (&result.previous).into(),
    })
}

pub fn income_delete(db: &mut Db, id: IncomeId, now: DateTime<Utc>) -> Result<IncomeDto, AppError> {
    let before = db.income(id).map_err(income_error(id))?;
    db.income_delete(id, now).map_err(income_error(id))?;
    Ok((&before).into())
}

pub fn income_restore(
    db: &mut Db,
    id: IncomeId,
    now: DateTime<Utc>,
) -> Result<IncomeDto, AppError> {
    let restored = db.income_restore(id, now).map_err(income_error(id))?;
    Ok((&restored).into())
}

pub fn tags_list(db: &Db) -> Result<Vec<TagDto>, AppError> {
    Ok(db.tags()?.iter().map(TagDto::from).collect())
}

pub fn tag_create(db: &mut Db, name: &str) -> Result<TagDto, AppError> {
    let tag = db.tag_create(name)?;
    Ok((&tag).into())
}

/// Месяцы, затронутые правкой: до и после (если запись перенесли в другой месяц).
pub fn touched_months(previous: &str, current: &str) -> Vec<String> {
    if previous == current {
        vec![current.to_owned()]
    } else {
        vec![previous.to_owned(), current.to_owned()]
    }
}

//! Категории, лимиты и план сбережений.

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::{BasisPoints, CategoryId, Money, YearMonth};
use planning_budget_storage::{CategoryPatch, Db, NewCategory};

use super::{month_of, parse_month};
use crate::AppError;
use crate::dto::{CategoryDto, CategoryInput, CategoryPatchDto, LimitEntryDto};

fn category_error(id: CategoryId) -> impl Fn(planning_budget_storage::StorageError) -> AppError {
    move |e| AppError::from_storage(e, "category", id.0)
}

pub fn list(db: &Db, include_archived: bool) -> Result<Vec<CategoryDto>, AppError> {
    Ok(db
        .categories(include_archived)?
        .iter()
        .map(CategoryDto::from)
        .collect())
}

pub fn create(
    db: &mut Db,
    input: CategoryInput,
    now: DateTime<Utc>,
) -> Result<CategoryDto, AppError> {
    let created = db.category_create(
        &NewCategory {
            name: input.name,
            kind: input.kind.into(),
            color: input.color,
            note: input.note,
        },
        now,
    )?;
    Ok((&created).into())
}

pub fn update(
    db: &mut Db,
    id: CategoryId,
    patch: CategoryPatchDto,
    now: DateTime<Utc>,
) -> Result<CategoryDto, AppError> {
    let updated = db
        .category_update(
            id,
            &CategoryPatch {
                name: patch.name,
                color: patch.color,
                note: patch.note,
            },
            now,
        )
        .map_err(category_error(id))?;
    Ok((&updated).into())
}

/// Архивирует категорию; `today` определяет месяц, с которого процент плана сбережений
/// становится нулевым.
pub fn archive(
    db: &mut Db,
    id: CategoryId,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    db.category_archive(id, month_of(today)?, now)
        .map_err(category_error(id))
}

pub fn unarchive(db: &mut Db, id: CategoryId, now: DateTime<Utc>) -> Result<(), AppError> {
    db.category_unarchive(id, now).map_err(category_error(id))
}

pub fn delete(db: &mut Db, id: CategoryId) -> Result<(), AppError> {
    db.category_delete(id).map_err(category_error(id))
}

pub fn reorder(
    db: &mut Db,
    ids: Vec<i64>,
    now: DateTime<Utc>,
) -> Result<Vec<CategoryDto>, AppError> {
    let ids: Vec<CategoryId> = ids.into_iter().map(CategoryId).collect();
    db.categories_reorder(&ids, now)?;
    list(db, false)
}

pub fn limits_set(
    db: &mut Db,
    category: CategoryId,
    valid_from: &str,
    amount: i64,
) -> Result<(), AppError> {
    let valid_from = parse_month(valid_from, "validFrom")?;
    db.limit_set(category, valid_from, Money::from_kopecks(amount))
        .map_err(category_error(category))
}

pub fn limits_unset(db: &mut Db, category: CategoryId, valid_from: &str) -> Result<(), AppError> {
    let valid_from = parse_month(valid_from, "validFrom")?;
    db.limit_unset(category, valid_from)
        .map_err(category_error(category))
}

pub fn limits_clear(db: &mut Db, category: CategoryId, valid_from: &str) -> Result<(), AppError> {
    let valid_from = parse_month(valid_from, "validFrom")?;
    db.limit_clear(category, valid_from)
        .map_err(category_error(category))
}

pub fn limits_history(db: &Db, category: CategoryId) -> Result<Vec<LimitEntryDto>, AppError> {
    Ok(db
        .limit_history(category)?
        .iter()
        .map(|e| LimitEntryDto {
            valid_from: e.valid_from.to_string(),
            amount: e.amount.map(Money::kopecks),
        })
        .collect())
}

fn rate(rate_bp: i32) -> BasisPoints {
    BasisPoints(rate_bp)
}

pub fn savings_rate_set(
    db: &mut Db,
    category: CategoryId,
    valid_from: &str,
    rate_bp: i32,
) -> Result<(), AppError> {
    let valid_from = parse_month(valid_from, "validFrom")?;
    db.savings_rate_set(category, valid_from, rate(rate_bp))
        .map_err(category_error(category))
}

pub fn savings_override_set(
    db: &mut Db,
    month: &str,
    category: CategoryId,
    rate_bp: i32,
) -> Result<YearMonth, AppError> {
    let month = parse_month(month, "month")?;
    db.savings_override_set(month, category, rate(rate_bp))
        .map_err(category_error(category))?;
    Ok(month)
}

pub fn savings_override_clear(
    db: &mut Db,
    month: &str,
    category: CategoryId,
) -> Result<YearMonth, AppError> {
    let month = parse_month(month, "month")?;
    db.savings_override_clear(month, category)
        .map_err(category_error(category))?;
    Ok(month)
}

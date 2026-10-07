//! Общие части записей трат и доходов: источник, месяц и дата, проверки.

use chrono::{Datelike as _, NaiveDate};
use planning_budget_core::{CategoryId, Money, YearMonth};

use crate::{Db, StorageError};

/// Откуда взялась запись.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RecordSource {
    Manual,
    Import,
    Legacy,
    Seed,
}

impl RecordSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Import => "import",
            Self::Legacy => "legacy",
            Self::Seed => "seed",
        }
    }

    pub(crate) fn parse(s: &str) -> Result<Self, StorageError> {
        match s {
            "manual" => Ok(Self::Manual),
            "import" => Ok(Self::Import),
            "legacy" => Ok(Self::Legacy),
            "seed" => Ok(Self::Seed),
            _ => Err(StorageError::Corrupt),
        }
    }
}

pub(crate) fn parse_month(s: &str) -> Result<YearMonth, StorageError> {
    YearMonth::parse(s).map_err(|_| StorageError::Corrupt)
}

pub(crate) fn parse_date(s: &str) -> Result<NaiveDate, StorageError> {
    s.parse().map_err(|_| StorageError::Corrupt)
}

pub(crate) fn month_of(date: NaiveDate) -> Result<YearMonth, StorageError> {
    let year =
        u16::try_from(date.year()).map_err(|_| StorageError::Invalid("record.date_invalid"))?;
    let month = u8::try_from(date.month()).map_err(|_| StorageError::Corrupt)?;
    YearMonth::new(year, month).map_err(|_| StorageError::Invalid("record.date_invalid"))
}

/// Месяц и дата записи при создании: дата, если она есть, определяет месяц.
pub(crate) fn period(
    month: YearMonth,
    date: Option<NaiveDate>,
) -> Result<(YearMonth, Option<NaiveDate>), StorageError> {
    if let Some(date) = date
        && month_of(date)? != month
    {
        return Err(StorageError::Invalid("record.date_month_mismatch"));
    }
    Ok((month, date))
}

/// Месяц и дата после правки. Смена месяца без новой даты сбрасывает старую дату,
/// смена даты без месяца переносит запись в месяц новой даты.
pub(crate) fn patched_period(
    old: (YearMonth, Option<NaiveDate>),
    month: Option<YearMonth>,
    date: Option<Option<NaiveDate>>,
) -> Result<(YearMonth, Option<NaiveDate>), StorageError> {
    let new_date = match date {
        Some(date) => date,
        None if month.is_some_and(|m| m != old.0) => None,
        None => old.1,
    };
    let new_month = match (month, new_date) {
        (Some(m), _) => m,
        (None, Some(d)) if date.is_some() => month_of(d)?,
        (None, _) => old.0,
    };
    period(new_month, new_date)
}

/// Верхняя граница суммы записи: 10^12 ₽ в копейках. Больше не бывает, а значения
/// около `i64::MAX` ломали бы сложение в расчётах и выходили бы за 2^53 в UI.
const MAX_AMOUNT_KOPECKS: i64 = 100_000_000_000_000;

pub(crate) fn validate_amount(amount: Money) -> Result<(), StorageError> {
    if amount <= Money::ZERO {
        Err(StorageError::Invalid("record.amount_not_positive"))
    } else if amount.kopecks() > MAX_AMOUNT_KOPECKS {
        Err(StorageError::Invalid("record.amount_too_large"))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_title(title: &str) -> Result<&str, StorageError> {
    let title = title.trim();
    if title.is_empty() {
        Err(StorageError::Invalid("record.title_empty"))
    } else {
        Ok(title)
    }
}

impl Db {
    /// Категория, в которую можно записывать: существует и не в архиве.
    pub(crate) fn require_active_category(&self, id: CategoryId) -> Result<(), StorageError> {
        if self.category(id)?.archived {
            return Err(StorageError::Invalid("record.category_archived"));
        }
        Ok(())
    }
}

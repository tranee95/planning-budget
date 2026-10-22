//! Общие части записей трат и доходов: источник, месяц и дата, проверки.

use chrono::NaiveDate;
use planning_budget_core::{CategoryId, YearMonth};

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

pub(crate) use planning_budget_core::records::{
    month_of, patched_period, period, validate_amount, validate_title,
};

impl Db {
    /// Категория, в которую можно записывать: существует и не в архиве.
    pub(crate) fn require_active_category(&self, id: CategoryId) -> Result<(), StorageError> {
        if self.category(id)?.archived {
            return Err(StorageError::Invalid("record.category_archived"));
        }
        Ok(())
    }
}

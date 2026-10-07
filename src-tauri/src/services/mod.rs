//! Сервисы: логика команд без Tauri-типов, чтобы тестироваться напрямую.

pub mod analytics;
pub mod categories;
#[cfg(test)]
mod checklist;
#[cfg(test)]
mod data_tests;
pub mod debts;
pub mod devseed;
pub mod legacy;
pub mod plan;
pub mod records;
pub mod savings;
pub mod search;
pub mod settings;
pub mod summary;
pub mod vault;

use chrono::{Datelike as _, NaiveDate};
use planning_budget_core::YearMonth;

use crate::AppError;

/// Месяц из строки `YYYY-MM`, пришедшей с фронтенда.
pub(crate) fn parse_month(value: &str, field: &str) -> Result<YearMonth, AppError> {
    YearMonth::parse(value).map_err(|_| AppError::invalid_field("month_invalid", field))
}

/// Дата из строки `YYYY-MM-DD`, пришедшей с фронтенда.
pub(crate) fn parse_date(value: &str, field: &str) -> Result<NaiveDate, AppError> {
    value
        .parse()
        .map_err(|_| AppError::invalid_field("date_invalid", field))
}

/// Месяц, в который попадает дата.
pub(crate) fn month_of(date: NaiveDate) -> Result<YearMonth, AppError> {
    let year =
        u16::try_from(date.year()).map_err(|_| AppError::invalid_field("date_invalid", "date"))?;
    let month =
        u8::try_from(date.month()).map_err(|_| AppError::invalid_field("date_invalid", "date"))?;
    YearMonth::new(year, month).map_err(|_| AppError::invalid_field("date_invalid", "date"))
}

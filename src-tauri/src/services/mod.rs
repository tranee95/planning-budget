//! Сервисы: логика команд без Tauri-типов, чтобы тестироваться напрямую.

pub mod analytics;
pub mod categories;
#[cfg(test)]
mod checklist;
pub mod client_log;
#[cfg(test)]
mod data_tests;
pub mod debts;
#[cfg(test)]
mod debts_tests;
pub mod devseed;
pub mod legacy;
pub mod plan;
#[cfg(test)]
mod plan_tests;
pub mod records;
#[cfg(test)]
mod records_tests;
pub mod savings;
#[cfg(test)]
mod savings_tests;
pub mod search;
pub mod settings;
pub mod summary;
#[cfg(test)]
mod summary_tests;
#[cfg(test)]
mod test_support;
pub mod updates;
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

//! Перенос данных из старой таблицы xlsx.

use std::fs;
use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::YearMonth;
use planning_budget_core::calc::Ledger;
use planning_budget_import::parse_legacy;
use planning_budget_storage::Db;

use super::month_of;
use crate::AppError;
use crate::dto::{BlockMismatchDto, LegacyReportDto, LegacyTotalsDto};

/// Предел размера файла импорта: больше — не читаем.
const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

fn io_error(key: &str) -> AppError {
    AppError::Io {
        message_key: format!("errors.import.{key}"),
    }
}

/// Читает выбранный в диалоге файл: расширение `.xlsx`, размер не больше 50 МБ.
/// Путь в сообщениях и логах не используется.
pub fn read_file(path: &Path) -> Result<Vec<u8>, AppError> {
    let is_xlsx = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"));
    if !is_xlsx {
        return Err(io_error("wrong_extension"));
    }
    let size = fs::metadata(path)
        .map_err(|_| io_error("unreadable"))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(io_error("too_large"));
    }
    fs::read(path).map_err(|_| io_error("unreadable"))
}

/// Разбирает книгу, записывает её в базу и считает итоги года для сверки.
/// `today` определяет «текущий месяц» в итогах: будущие месяцы в них не входят.
pub fn import(
    db: &mut Db,
    bytes: &[u8],
    now: DateTime<Utc>,
    today: NaiveDate,
) -> Result<LegacyReportDto, AppError> {
    let book = parse_legacy(bytes)?;
    let report = db.import_legacy(&book, now)?;
    let year = match book.first_month() {
        Some(first) => first.year(),
        None => month_of(today)?.year(),
    };
    // Данные уже записаны: сбой подсчёта итогов не должен выдавать импорт за неудавшийся.
    let totals = year_totals(db, year, today)
        .inspect_err(|err| tracing::warn!(%err, "legacy import totals failed"))
        .ok();
    Ok(LegacyReportDto {
        categories_created: report.categories_created,
        categories_matched: report.categories_matched,
        kind_conflicts: report.kind_conflicts,
        limits_written: report.limits_written,
        transactions: report.transactions,
        incomes: report.incomes,
        settings_applied: report.settings_applied,
        block_mismatches: book
            .block_mismatches
            .iter()
            .map(|m| BlockMismatchDto {
                month: m.month.to_string(),
                category: m.category.clone(),
                declared: m.declared.kopecks(),
                parsed: m.parsed.kopecks(),
            })
            .collect(),
        totals,
    })
}

fn year_totals(db: &Db, year: u16, today: NaiveDate) -> Result<LegacyTotalsDto, AppError> {
    let data = db.dataset(YearMonth::first_of_year(year)?, YearMonth::new(year, 12)?)?;
    let summary = Ledger::new(&data)?.year_summary(year, month_of(today)?)?;
    Ok(LegacyTotalsDto {
        year,
        income: summary.income.kopecks(),
        expenses: summary.expenses.kopecks(),
        savings: summary.savings.kopecks(),
        free: summary.free.kopecks(),
        months_with_data: summary.months_with_data,
    })
}

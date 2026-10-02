//! Сводки месяца и года поверх `core::calc`.

use budget_core::YearMonth;
use budget_core::calc::Ledger;
use budget_storage::Db;
use chrono::NaiveDate;

use super::{month_of, parse_month};
use crate::AppError;
use crate::dto::{
    BudgetBalanceDto, CategoryYearRowDto, LimitRowDto, MonthOverviewDto, MonthSummaryDto,
    YearSummaryDto,
};

/// «Обзор» месяца: сводка и лимиты по категориям. Данные читаются с января года месяца:
/// от них зависит накопительный остаток.
pub fn month(db: &Db, month: &str) -> Result<MonthOverviewDto, AppError> {
    let month = parse_month(month, "month")?;
    let data = db.dataset(YearMonth::first_of_year(month.year())?, month)?;
    let ledger = Ledger::new(&data)?;
    let summary = ledger.month_summary(month)?;
    let limits = ledger.month_limits(month)?;
    Ok(MonthOverviewDto {
        summary: (&summary).into(),
        limits: limits.rows.iter().map(LimitRowDto::from).collect(),
        limits_total: limits.limits_total.kopecks(),
        limits_remaining: limits.limits_remaining.kopecks(),
        spent_vs_limits: limits.spent_vs_limits,
    })
}

/// Все строки листа «Сводка» за год. `today` определяет «текущий месяц»: будущие месяцы не
/// входят в итоги и средние, а лимиты в годовых сравнениях берутся на этот месяц.
pub fn year(db: &Db, year: u16, today: NaiveDate) -> Result<YearSummaryDto, AppError> {
    let current = month_of(today)?;
    let first = YearMonth::first_of_year(year)?;
    let last = YearMonth::new(year, 12)?;
    let data = db.dataset(first, last)?;
    let ledger = Ledger::new(&data)?;
    let summary = ledger.year_summary(year, current)?;
    let months = first
        .iter_to(last)
        .map(|m| ledger.month_summary(m).map(|s| MonthSummaryDto::from(&s)))
        .collect::<Result<Vec<_>, _>>()?;
    let categories = ledger.category_year_rows(year, current)?;
    let balance = ledger.budget_balance(year, current)?;
    Ok(YearSummaryDto {
        year,
        months,
        income: summary.income.kopecks(),
        expenses: summary.expenses.kopecks(),
        savings: summary.savings.kopecks(),
        free: summary.free.kopecks(),
        months_with_data: summary.months_with_data,
        avg_income: summary.avg_income.kopecks(),
        avg_expenses: summary.avg_expenses.kopecks(),
        avg_savings: summary.avg_savings.kopecks(),
        avg_free: summary.avg_free.kopecks(),
        savings_rate: summary.savings_rate,
        by_status: summary.by_status.into(),
        by_kind: summary.by_kind.into(),
        categories: categories.iter().map(CategoryYearRowDto::from).collect(),
        balance: BudgetBalanceDto::from(&balance),
    })
}

//! Сводки месяца и года поверх `core::calc`.

use chrono::NaiveDate;
use planning_budget_core::YearMonth;
use planning_budget_core::calc::Ledger;
use planning_budget_storage::Db;

use super::{month_of, parse_month};
use crate::AppError;
use crate::dto::{
    BudgetBalanceDto, CategoryYearRowDto, LimitRowDto, MonthOverviewDto, MonthSummaryDto,
    SeriesRangeDto, YearSummaryDto,
};

/// «Обзор» месяца: сводка и лимиты по категориям. Данные читаются с января года месяца:
/// от них зависит накопительный остаток. Для отклонения расходов от среднего за год читаются
/// и месяцы до `today` того же года (годовое среднее считается по месяцам не позже текущего).
pub fn month(db: &Db, month: &str, today: NaiveDate) -> Result<MonthOverviewDto, AppError> {
    let month = parse_month(month, "month")?;
    let current = month_of(today)?;
    let last = if current.year() > month.year() {
        YearMonth::new(month.year(), 12)?
    } else if current.year() == month.year() && current > month {
        current
    } else {
        month
    };
    let data = db.dataset(YearMonth::first_of_year(month.year())?, last)?;
    let ledger = Ledger::new(&data)?;
    let summary = ledger.month_summary(month)?;
    let limits = ledger.month_limits(month)?;
    let expenses_delta_percent = ledger
        .year_summary(month.year(), current)?
        .expenses_delta_percent(summary.expenses);
    Ok(MonthOverviewDto {
        summary: (&summary).into(),
        limits: limits.rows.iter().map(LimitRowDto::from).collect(),
        limits_total: limits.limits_total.kopecks(),
        limits_remaining: limits.limits_remaining.kopecks(),
        spent_vs_limits: limits.spent_vs_limits,
        over_count: limits.over_count,
        plan_locked: db.plan_is_locked(month)?,
        expenses_delta_percent,
    })
}

/// Сколько лет назад заглядывает период «Всё»: хватает на любую личную историю.
const MAX_YEARS_BACK: u16 = 10;

/// Ряд месяцев для графика «Обзора». Накопительные значения каждого
/// месяца считаются с января его года, поэтому данные читаются с января первого года ряда.
pub fn series(
    db: &Db,
    month: &str,
    range: SeriesRangeDto,
) -> Result<Vec<MonthSummaryDto>, AppError> {
    let month = parse_month(month, "month")?;
    let year = month.year();
    let (from, to) = match range {
        SeriesRangeDto::Year => (YearMonth::first_of_year(year)?, YearMonth::new(year, 12)?),
        SeriesRangeDto::Last12 => ((0..11).fold(month, |m, _| m.pred().unwrap_or(m)), month),
        SeriesRangeDto::All => (
            YearMonth::first_of_year(year.saturating_sub(MAX_YEARS_BACK).max(1970))?,
            YearMonth::new(year, 12)?,
        ),
    };
    let data = db.dataset(YearMonth::first_of_year(from.year())?, to)?;
    let ledger = Ledger::new(&data)?;
    let mut months = Vec::new();
    for m in from.iter_to(to) {
        let summary = ledger.month_summary(m)?;
        if range == SeriesRangeDto::All && summary.income.is_zero() && summary.expenses.is_zero() {
            continue;
        }
        months.push(MonthSummaryDto::from(&summary));
    }
    Ok(months)
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

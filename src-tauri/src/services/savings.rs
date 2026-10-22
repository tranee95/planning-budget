//! Раздел «Сбережения» поверх `core::calc` и хранилища.

use planning_budget_core::calc::Ledger;
use planning_budget_core::{CategoryId, Money, YearMonth};
use planning_budget_storage::Db;

use super::parse_month;
use crate::AppError;
use crate::dto::{
    ChartDataDto, ChartSeriesDto, ChartUnitDto, SavingsProgressCategoryDto, SavingsProgressDto,
    SavingsProgressMonthDto,
};

fn rub(m: Money) -> Option<f64> {
    Some(m.as_f64() / 100.0)
}

/// «Янв» — первые три буквы названия месяца.
fn short_month(month: YearMonth) -> String {
    month.month_name_ru().chars().take(3).collect()
}

pub fn fixed_set(
    db: &mut Db,
    category_id: i64,
    valid_from: &str,
    amount: i64,
) -> Result<(), AppError> {
    db.savings_fixed_set(
        CategoryId(category_id),
        parse_month(valid_from, "validFrom")?,
        Money::from_kopecks(amount),
    )?;
    Ok(())
}

/// Накопленное за весь период и взносы по месяцам `month`.
pub fn progress(db: &Db, month: &str) -> Result<SavingsProgressDto, AppError> {
    let current = parse_month(month, "month")?;
    let from = db
        .savings_first_deposit_month()?
        .unwrap_or(current)
        .min(current);
    let data = db.dataset(from, current)?;
    let report = Ledger::new(&data)?.savings_progress(current)?;
    let months: Vec<SavingsProgressMonthDto> = report
        .months
        .iter()
        .map(|m| SavingsProgressMonthDto {
            month: m.month.to_string(),
            deposited: m.deposited.kopecks(),
            cumulative: m.cumulative.kopecks(),
        })
        .collect();
    let known: std::collections::BTreeMap<i64, &planning_budget_core::Category> =
        data.categories.iter().map(|c| (c.id.0, c)).collect();
    let categories: Vec<SavingsProgressCategoryDto> = report
        .categories
        .iter()
        .filter_map(|c| {
            let category = known.get(&c.category_id.0)?;
            Some(SavingsProgressCategoryDto {
                category_id: c.category_id.0,
                name: category.name.clone(),
                color: category.color.clone(),
                archived: category.archived,
                total: c.total.kopecks(),
            })
        })
        .collect();
    // Ряд на каждое накопление: график показывает, из чего состоит каждый месяц.
    let series: Vec<ChartSeriesDto> = report
        .categories
        .iter()
        .filter_map(|c| {
            let category = known.get(&c.category_id.0)?;
            Some(ChartSeriesDto {
                name: category.name.clone(),
                color: format!("category:{}", c.category_id.0),
                values: c.amounts.iter().map(|m| rub(*m)).collect(),
            })
        })
        .collect();
    let chart = ChartDataDto {
        categories: report
            .months
            .iter()
            .map(|m| format!("{} {}", short_month(m.month), m.month.year()))
            .collect(),
        category_tokens: months
            .iter()
            .map(|m| format!("month:{}", m.month))
            .collect(),
        series,
        reference_lines: Vec::new(),
        totals: None,
        unit: ChartUnitDto::Rub,
    };
    Ok(SavingsProgressDto {
        months,
        total: report.total.kopecks(),
        categories,
        chart,
    })
}

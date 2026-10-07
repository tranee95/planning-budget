//! Лимиты категорий, годовые сравнения и «Баланс бюджета».

use std::collections::BTreeSet;

use super::{Ledger, StatusAmounts, YearSummary};
use crate::error::CoreError;
use crate::model::{CategoryId, CategoryKind};
use crate::money::Money;
use crate::period::YearMonth;

/// Уровень использования лимита.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LimitLevel {
    /// Меньше 85% лимита.
    Ok,
    /// От 85% до 100% включительно.
    Warn,
    /// Больше лимита; при нулевом лимите — любая трата.
    Over,
}

/// Категория в месяце: факт против лимита.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LimitRow {
    pub category_id: CategoryId,
    pub fact: Money,
    pub limit: Option<Money>,
    pub remaining: Option<Money>,
    /// `fact / limit`; при нулевом лимите 1 при `fact > 0`, иначе 0.
    pub usage: Option<f64>,
    /// `usage` в целых процентах; при нулевом лимите 100 при `fact > 0`, иначе 0.
    pub usage_percent: Option<i32>,
    /// Только у категорий-сбережений: процент плана по истории на этот месяц (без ручного значения
    /// месяца); `None`, если до месяца процент не задавался.
    pub plan_rate_bp: Option<i32>,
    /// Доля лимита, занятая оплаченным (всё, кроме статуса «План»); `None` без лимита или при нулевом лимите.
    pub paid_usage: Option<f64>,
    /// Доля лимита, занятая планом.
    pub planned_usage: Option<f64>,
    /// Для сбережений не задаётся: перевыполнение плана — не перерасход.
    pub level: Option<LimitLevel>,
    pub by_status: StatusAmounts,
}

/// Итоги месяца по лимитам.
#[derive(Clone, PartialEq, Debug)]
pub struct MonthLimits {
    pub month: YearMonth,
    pub rows: Vec<LimitRow>,
    /// Сумма заданных лимитов неархивных категорий, кроме сбережений.
    pub limits_total: Money,
    pub expenses: Money,
    pub spent_vs_limits: Option<f64>,
    pub limits_remaining: Money,
    /// Сколько категорий вышло за лимит (`level = Over`).
    pub over_count: u32,
}

/// Строка годового сравнения категорий.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CategoryYearRow {
    pub category_id: CategoryId,
    pub kind: CategoryKind,
    pub total: Money,
    pub avg: Money,
    pub limit_now: Option<Money>,
    /// Не считается для сбережений; больше нуля — выше лимита.
    pub avg_minus_limit: Option<Money>,
    pub share_in_expenses: Option<f64>,
}

/// «Баланс бюджета».
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BudgetBalance {
    pub avg_income: Money,
    pub savings_target: Money,
    pub limits_sum: Money,
    pub buffer: Money,
    pub buffer_rate: Option<f64>,
    pub economy: Money,
}

impl Ledger<'_> {
    /// Лимит категории на месяц. Сбережения — план самой категории; остальные — последняя строка
    /// истории с `valid_from ≤ month`; нет строки — лимита нет.
    pub fn limit(
        &self,
        month: YearMonth,
        category: CategoryId,
    ) -> Result<Option<Money>, CoreError> {
        let kind = self
            .kind_of(category)
            .ok_or(CoreError::CategoryNotFound(category.0))?;
        match kind {
            CategoryKind::Savings => Ok(Some(self.category_savings_plan(month, category)?)),
            CategoryKind::Mandatory | CategoryKind::Wants | CategoryKind::Loans => Ok(self
                .limit_history
                .get(&category)
                .and_then(|history| history.range(..=month).next_back())
                .map(|(_, amount)| *amount)),
        }
    }

    /// Лимиты и факт по категориям месяца. Архивные категории без трат в месяце пропускаются.
    pub fn month_limits(&self, month: YearMonth) -> Result<MonthLimits, CoreError> {
        let mut rows = Vec::with_capacity(self.data().categories.len());
        let mut limits_total = Money::ZERO;
        for category in &self.data().categories {
            let by_status = self.category_by_status(month, category.id);
            let fact = by_status.total()?;
            if category.archived && fact.is_zero() {
                continue;
            }
            let limit = self.limit(month, category.id)?;
            let is_savings = category.kind == CategoryKind::Savings;
            if let (Some(limit), false) = (limit, is_savings || category.archived) {
                limits_total = limits_total.checked_add(limit)?;
            }
            let paid = fact.checked_sub(by_status.planned)?;
            rows.push(LimitRow {
                category_id: category.id,
                fact,
                limit,
                remaining: limit.map(|l| l.checked_sub(fact)).transpose()?,
                usage: limit.map(|l| usage(fact, l)),
                usage_percent: limit.map(|l| usage_percent(fact, l)),
                plan_rate_bp: if is_savings {
                    self.category_rate_from_history(month, category.id)
                        .map(|r| r.0)
                } else {
                    None
                },
                paid_usage: limit
                    .filter(|l| !l.is_zero())
                    .map(|l| paid.as_f64() / l.as_f64()),
                planned_usage: limit
                    .filter(|l| !l.is_zero())
                    .map(|l| by_status.planned.as_f64() / l.as_f64()),
                level: if is_savings {
                    None
                } else {
                    limit.map(|l| level(fact, l))
                },
                by_status,
            });
        }
        let expenses = self.totals(month).by_kind.expenses()?;
        let over_count = rows
            .iter()
            .filter(|r| r.level == Some(LimitLevel::Over))
            .count();
        Ok(MonthLimits {
            month,
            rows,
            limits_total,
            expenses,
            spent_vs_limits: expenses.ratio(limits_total),
            limits_remaining: limits_total.checked_sub(expenses)?,
            over_count: u32::try_from(over_count).unwrap_or(u32::MAX),
        })
    }

    /// Годовое сравнение категорий. `current` — месяц, чей лимит считается действующим.
    pub fn category_year_rows(
        &self,
        year: u16,
        current: YearMonth,
    ) -> Result<Vec<CategoryYearRow>, CoreError> {
        self.category_rows(&self.year_summary(year, current)?, current)
    }

    fn category_rows(
        &self,
        summary: &YearSummary,
        current: YearMonth,
    ) -> Result<Vec<CategoryYearRow>, CoreError> {
        let year = summary.year;
        let divisor = i64::from(summary.months_with_data);
        let mut rows = Vec::with_capacity(self.data().categories.len());
        for category in &self.data().categories {
            let total = self.category_year_total(year, current, category.id)?;
            if category.archived && total.is_zero() {
                continue;
            }
            let avg = total.div_round(divisor)?;
            let limit_now = self.limit(current, category.id)?;
            let is_savings = category.kind == CategoryKind::Savings;
            rows.push(CategoryYearRow {
                category_id: category.id,
                kind: category.kind,
                total,
                avg,
                limit_now,
                avg_minus_limit: match (limit_now, is_savings) {
                    (Some(limit), false) => Some(avg.checked_sub(limit)?),
                    _ => None,
                },
                share_in_expenses: if is_savings {
                    None
                } else {
                    total.ratio(summary.expenses)
                },
            });
        }
        Ok(rows)
    }

    /// «Баланс бюджета» года `year` при лимитах месяца `current`.
    ///
    /// Архивные категории не входят: их лимит больше не действует. `economy` суммирует
    /// превышение среднего над лимитом только по категориям с заданным лимитом: у категории
    /// без лимита сравнивать не с чем.
    pub fn budget_balance(
        &self,
        year: u16,
        current: YearMonth,
    ) -> Result<BudgetBalance, CoreError> {
        self.balance_for(&self.year_summary(year, current)?, current)
    }

    pub(crate) fn balance_for(
        &self,
        summary: &YearSummary,
        current: YearMonth,
    ) -> Result<BudgetBalance, CoreError> {
        let avg_income = summary.avg_income;
        let savings_target = avg_income.mul_ratio(self.data().settings.savings_norm.as_ratio())?;

        let mut limits_sum = Money::ZERO;
        let mut economy = Money::ZERO;
        let archived: BTreeSet<CategoryId> = self
            .data()
            .categories
            .iter()
            .filter(|c| c.archived)
            .map(|c| c.id)
            .collect();
        for row in self.category_rows(summary, current)? {
            if archived.contains(&row.category_id) {
                continue;
            }
            match row.kind {
                CategoryKind::Savings => {}
                CategoryKind::Loans => {
                    limits_sum = limits_sum.checked_add(row.limit_now.unwrap_or_default())?;
                }
                CategoryKind::Mandatory | CategoryKind::Wants => {
                    limits_sum = limits_sum.checked_add(row.limit_now.unwrap_or_default())?;
                    economy = economy
                        .checked_add(row.avg_minus_limit.unwrap_or_default().max(Money::ZERO))?;
                }
            }
        }
        let buffer = avg_income
            .checked_sub(savings_target)?
            .checked_sub(limits_sum)?;
        Ok(BudgetBalance {
            avg_income,
            savings_target,
            limits_sum,
            buffer,
            buffer_rate: buffer.ratio(avg_income),
            economy,
        })
    }
}

pub(crate) fn usage(fact: Money, limit: Money) -> f64 {
    if limit.is_zero() {
        return if fact > Money::ZERO { 1.0 } else { 0.0 };
    }
    fact.as_f64() / limit.as_f64()
}

fn usage_percent(fact: Money, limit: Money) -> i32 {
    if limit.is_zero() {
        return if fact > Money::ZERO { 100 } else { 0 };
    }
    fact.ratio_percent(limit).unwrap_or(0)
}

/// Границы сравниваются в целых числах: ровно 85% и ровно 100% не зависят от `f64`.
fn level(fact: Money, limit: Money) -> LimitLevel {
    if fact.is_zero() {
        LimitLevel::Ok
    } else if fact > limit {
        LimitLevel::Over
    } else if i128::from(fact.kopecks()) * 100 < i128::from(limit.kopecks()) * 85 {
        LimitLevel::Ok
    } else {
        LimitLevel::Warn
    }
}

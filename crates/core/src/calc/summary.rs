//! Сводка месяца и итоги года.

use super::{KindAmounts, Ledger, StatusAmounts};
use crate::error::{CoreError, MoneyError};
use crate::model::{BasisPoints, CategoryId, CategoryKind, TxStatus};
use crate::money::Money;
use crate::period::YearMonth;

/// Положение нормы сбережений относительно коридора.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Corridor {
    NoIncome,
    Below,
    Within,
    Above,
}

/// Сводка месяца (лист «Сводка», строки 4–15, статусы и типы).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MonthSummary {
    pub month: YearMonth,
    pub income: Money,
    pub income_received: Money,
    pub income_expected: Money,
    /// Расходы без сбережений; статус «План» входит (так считает таблица).
    pub expenses: Money,
    pub savings: Money,
    pub free: Money,
    pub free_cum: Money,
    pub savings_cum: Money,
    pub savings_rate: Option<f64>,
    pub unspent_rate: Option<f64>,
    pub savings_plan_rate: BasisPoints,
    pub savings_plan: Money,
    /// Факт − план; отрицательное значение — недобор.
    pub savings_gap: Money,
    pub per_week: Money,
    pub corridor: Corridor,
    /// Сколько доложить до нижней границы коридора.
    pub top_up_to_min: Money,
    /// Сколько доложить до нормы.
    pub top_up_to_norm: Money,
    /// Включая категории сбережений.
    pub by_status: StatusAmounts,
    pub by_kind: KindAmounts,
}

/// Итоги года.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct YearSummary {
    pub year: u16,
    pub income: Money,
    pub expenses: Money,
    pub savings: Money,
    pub free: Money,
    /// Месяцы года не позже текущего с доходом > 0 (любой статус), но не меньше 1: делитель
    /// всех «средних в месяц».
    pub months_with_data: u32,
    pub avg_income: Money,
    pub avg_expenses: Money,
    pub avg_savings: Money,
    pub avg_free: Money,
    pub savings_rate: Option<f64>,
    pub by_status: StatusAmounts,
    pub by_kind: KindAmounts,
}

impl YearSummary {
    /// Доля статуса во всех тратах года.
    pub fn status_share(&self, status: TxStatus) -> Result<Option<f64>, MoneyError> {
        Ok(self.by_status.get(status).ratio(self.by_status.total()?))
    }

    /// Доля типа категорий в доходе года.
    pub fn kind_share_of_income(&self, kind: CategoryKind) -> Option<f64> {
        self.by_kind.get(kind).ratio(self.income)
    }
}

impl Ledger<'_> {
    /// Сводка месяца `month`; накопительные значения считаются с января того же года.
    pub fn month_summary(&self, month: YearMonth) -> Result<MonthSummary, CoreError> {
        let settings = &self.data().settings;
        let totals = self.totals(month);
        let income = totals.income()?;
        let expenses = totals.by_kind.expenses()?;
        let savings = totals.by_kind.savings;
        let free = totals.free()?;

        let mut free_cum = Money::ZERO;
        let mut savings_cum = Money::ZERO;
        for m in YearMonth::first_of_year(month.year())?.iter_to(month) {
            let t = self.totals(m);
            free_cum = free_cum.checked_add(t.free()?)?;
            savings_cum = savings_cum.checked_add(t.by_kind.savings)?;
        }

        let plan_rate = self.savings_plan_rate(month);
        let savings_plan = self.savings_plan(month)?;
        let weeks = i64::from(settings.weeks_per_month);

        Ok(MonthSummary {
            month,
            income,
            income_received: totals.income_received,
            income_expected: totals.income_expected,
            expenses,
            savings,
            free,
            free_cum,
            savings_cum,
            savings_rate: savings.ratio(income),
            unspent_rate: savings.checked_add(free)?.ratio(income),
            savings_plan_rate: plan_rate,
            savings_plan,
            savings_gap: savings.checked_sub(savings_plan)?,
            per_week: free.div_round(weeks)?,
            corridor: corridor(savings, income, settings.savings_min, settings.savings_max),
            top_up_to_min: top_up(savings, income, settings.savings_min)?,
            top_up_to_norm: top_up(savings, income, settings.savings_norm)?,
            by_status: totals.by_status,
            by_kind: totals.by_kind,
        })
    }

    /// Итоги года по месяцам не позже `current`: будущие месяцы не входят ни в суммы,
    /// ни в делитель средних `months_with_data`.
    pub fn year_summary(&self, year: u16, current: YearMonth) -> Result<YearSummary, CoreError> {
        let mut income = Money::ZERO;
        let mut by_status = StatusAmounts::default();
        let mut by_kind = KindAmounts::default();
        let mut months_with_income = 0_u32;
        let mut free = Money::ZERO;
        for month in YearMonth::months_of_year(year)?.filter(|m| *m <= current) {
            let totals = self.totals(month);
            let month_income = totals.income()?;
            if month_income > Money::ZERO {
                months_with_income = months_with_income.saturating_add(1);
            }
            income = income.checked_add(month_income)?;
            free = free.checked_add(totals.free()?)?;
            by_status.merge(&totals.by_status)?;
            by_kind.merge(&totals.by_kind)?;
        }
        let months_with_data = months_with_income.max(1);
        let divisor = i64::from(months_with_data);

        let expenses = by_kind.expenses()?;
        let savings = by_kind.savings;
        Ok(YearSummary {
            year,
            income,
            expenses,
            savings,
            free,
            months_with_data,
            avg_income: income.div_round(divisor)?,
            avg_expenses: expenses.div_round(divisor)?,
            avg_savings: savings.div_round(divisor)?,
            avg_free: free.div_round(divisor)?,
            savings_rate: savings.ratio(income),
            by_status,
            by_kind,
        })
    }

    /// Процент плана категории-сбережения: ручной на месяц, иначе последняя строка истории
    /// с `valid_from ≤ month`, иначе ноль.
    pub fn category_savings_rate(&self, month: YearMonth, category: CategoryId) -> BasisPoints {
        self.data()
            .savings_overrides
            .get(&(month, category))
            .copied()
            .or_else(|| {
                self.savings_rate_history
                    .get(&category)
                    .and_then(|history| history.range(..=month).next_back())
                    .map(|(_, rate)| *rate)
            })
            .unwrap_or(BasisPoints(0))
    }

    /// План категории-сбережения в рублях: `round(income × rate)`.
    pub fn category_savings_plan(
        &self,
        month: YearMonth,
        category: CategoryId,
    ) -> Result<Money, CoreError> {
        let income = self.totals(month).income()?;
        Ok(income.mul_ratio(self.category_savings_rate(month, category).as_ratio())?)
    }

    fn savings_categories(&self) -> impl Iterator<Item = CategoryId> + '_ {
        self.data()
            .categories
            .iter()
            .filter(|c| c.kind == CategoryKind::Savings)
            .map(|c| c.id)
    }

    /// Общий процент плана месяца: сумма процентов категорий-сбережений.
    pub fn savings_plan_rate(&self, month: YearMonth) -> BasisPoints {
        BasisPoints(
            self.savings_categories()
                .map(|id| self.category_savings_rate(month, id).0)
                .sum(),
        )
    }

    /// План сбережений месяца в рублях: сумма планов категорий.
    pub fn savings_plan(&self, month: YearMonth) -> Result<Money, CoreError> {
        self.savings_categories().try_fold(Money::ZERO, |sum, id| {
            Ok(sum.checked_add(self.category_savings_plan(month, id)?)?)
        })
    }
}

/// Сравнение в целых числах: на границе коридора `13,00%` не зависит от погрешности `f64`.
fn corridor(savings: Money, income: Money, min: BasisPoints, max: BasisPoints) -> Corridor {
    if income <= Money::ZERO {
        return Corridor::NoIncome;
    }
    let scaled_savings = i128::from(savings.kopecks()) * 10_000;
    let at = |bp: BasisPoints| i128::from(income.kopecks()) * i128::from(bp.0);
    if scaled_savings < at(min) {
        Corridor::Below
    } else if scaled_savings > at(max) {
        Corridor::Above
    } else {
        Corridor::Within
    }
}

/// Сколько доложить, чтобы доля сбережений достигла `target`. Цель округляется вверх до
/// копейки тем же целочисленным сравнением, что и в `corridor`: «ниже коридора» и
/// «доложить 0 ₽» одновременно получиться не могут.
fn top_up(savings: Money, income: Money, target: BasisPoints) -> Result<Money, MoneyError> {
    if income <= Money::ZERO || target.0 <= 0 {
        return Ok(Money::ZERO);
    }
    let scaled = i128::from(income.kopecks()) * i128::from(target.0);
    let wanted = i64::try_from((scaled + 9_999) / 10_000).map_err(|_| MoneyError::Overflow)?;
    Ok(Money::from_kopecks(wanted)
        .checked_sub(savings)?
        .max(Money::ZERO))
}

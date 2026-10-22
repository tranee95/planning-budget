//! Сводка месяца и итоги года.

use super::{KindAmounts, Ledger, PlanEntry, StatusAmounts};
use crate::error::{CoreError, MoneyError};
use crate::model::{BasisPoints, CategoryId, CategoryKind, TxStatus};
use crate::money::Money;
use crate::period::{CalendarDay, YearMonth};

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
    /// Долги, взятые в месяце (источник денег),.
    pub borrowed: Money,
    /// Оплаченные погашения месяца (выплата, не расход).
    pub repaid: Money,
    pub free: Money,
    /// «По факту»: `free` без записей со статусом «План».
    pub free_actual: Money,
    pub free_cum: Money,
    pub savings_cum: Money,
    pub savings_rate: Option<f64>,
    /// Норма сбережений в базисных пунктах (10 000 = 100 %); `None` без дохода.
    pub savings_rate_bp: Option<i32>,
    pub unspent_rate: Option<f64>,
    pub savings_plan_rate: BasisPoints,
    /// Процент плана больше нуля и отличается от нормы из настроек: UI объясняет расхождение.
    pub savings_plan_off_norm: bool,
    pub savings_plan: Money,
    /// Факт − план; отрицательное значение — недобор.
    pub savings_gap: Money,
    /// «На неделю»: остаток ÷ недели; в текущем месяце — ÷ оставшиеся недели.
    pub per_week: Money,
    /// Оставшиеся недели месяца (дни до конца месяца ÷ 7); только для текущего месяца.
    pub weeks_left: Option<f64>,
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

    /// Отклонение расходов месяца от среднего за год в целых процентах. `None`, если сравнивать не
    /// с чем: меньше двух месяцев с данными или среднее не больше нуля.
    pub fn expenses_delta_percent(&self, month_expenses: Money) -> Option<i32> {
        if self.months_with_data < 2 || self.avg_expenses <= Money::ZERO {
            return None;
        }
        month_expenses
            .checked_sub(self.avg_expenses)
            .ok()?
            .ratio_percent(self.avg_expenses)
    }

    /// Доля типа категорий в доходе года.
    pub fn kind_share_of_income(&self, kind: CategoryKind) -> Option<f64> {
        self.by_kind.get(kind).ratio(self.income)
    }
}

impl Ledger<'_> {
    /// Сводка месяца `month`; накопительные значения считаются с января того же года.
    /// «На неделю» — остаток ÷ `weeks_per_month`; без «сегодня» оставшихся недель нет.
    pub fn month_summary(&self, month: YearMonth) -> Result<MonthSummary, CoreError> {
        self.summary_of(month, None)
    }

    /// Сводка месяца с «сегодня»: если `today` в этом месяце, «На неделю» = остаток ÷ оставшиеся
    /// недели (дни от сегодня до конца месяца включительно ÷ 7, не меньше одного дня), иначе
    /// прежнее правило.
    pub fn month_summary_at(
        &self,
        month: YearMonth,
        today: CalendarDay,
    ) -> Result<MonthSummary, CoreError> {
        self.summary_of(month, Some(today).filter(|t| t.month() == month))
    }

    fn summary_of(
        &self,
        month: YearMonth,
        today: Option<CalendarDay>,
    ) -> Result<MonthSummary, CoreError> {
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
        let (per_week, weeks_left) = match today {
            Some(today) => {
                let days = today.days_left();
                let scaled = free.kopecks().checked_mul(7).ok_or(MoneyError::Overflow)?;
                let per_week = Money::from_kopecks(scaled).div_round(i64::from(days))?;
                (per_week, Some(f64::from(days) / 7.0))
            }
            None => (free.div_round(i64::from(settings.weeks_per_month))?, None),
        };

        Ok(MonthSummary {
            month,
            income,
            income_received: totals.income_received,
            income_expected: totals.income_expected,
            expenses,
            savings,
            borrowed: totals.borrowed,
            repaid: totals.repaid,
            free,
            free_actual: totals.free_actual()?,
            free_cum,
            savings_cum,
            savings_rate: savings.ratio(income),
            savings_rate_bp: savings.ratio_bp(income),
            unspent_rate: savings.checked_add(free)?.ratio(income),
            savings_plan_rate: plan_rate,
            savings_plan_off_norm: plan_rate.0 > 0 && plan_rate != settings.savings_norm,
            savings_plan,
            savings_gap: savings.checked_sub(savings_plan)?,
            per_week,
            weeks_left,
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

    /// Процент категории-сбережения по истории (без ручного значения месяца): последняя строка
    /// с `valid_from ≤ month`; `None`, если строк до месяца ещё нет.
    pub fn category_rate_from_history(
        &self,
        month: YearMonth,
        category: CategoryId,
    ) -> Option<BasisPoints> {
        self.plan_entry(month, category)
            .filter(|e| e.fixed.is_none())
            .map(|e| e.rate)
    }

    /// Процент плана категории-сбережения: ручной на месяц, иначе последняя строка истории
    /// с `valid_from ≤ month`, иначе ноль.
    pub fn category_savings_rate(&self, month: YearMonth, category: CategoryId) -> BasisPoints {
        self.data()
            .savings_overrides
            .get(&(month, category))
            .copied()
            .or_else(|| self.plan_entry(month, category).map(|e| e.rate))
            .unwrap_or(BasisPoints(0))
    }

    /// Действующая строка истории плана накопления на месяц.
    pub(crate) fn plan_entry(&self, month: YearMonth, category: CategoryId) -> Option<PlanEntry> {
        self.savings_rate_history
            .get(&category)
            .and_then(|history| history.range(..=month).next_back())
            .map(|(_, entry)| *entry)
    }

    /// План категории-сбережения в рублях: ручной процент месяца перекрывает всё; иначе
    /// фиксированная сумма действующей строки истории; иначе `round(income × rate)`.
    pub fn category_savings_plan(
        &self,
        month: YearMonth,
        category: CategoryId,
    ) -> Result<Money, CoreError> {
        let overridden = self
            .data()
            .savings_overrides
            .contains_key(&(month, category));
        if !overridden && let Some(fixed) = self.plan_entry(month, category).and_then(|e| e.fixed) {
            return Ok(fixed);
        }
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

/// Период ряда месяцев для графика «Обзора».
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeriesRange {
    /// Двенадцать месяцев года выбранного месяца.
    Year,
    /// Двенадцать месяцев, заканчивая выбранным.
    Last12,
    /// Все месяцы с данными, не дальше [`MAX_YEARS_BACK`] лет назад.
    All,
}

/// Сколько лет назад заглядывает период «Всё»: хватает на любую личную историю.
pub const MAX_YEARS_BACK: u16 = 10;

/// Последний месяц, который нужно загрузить для «Обзора» месяца `month` при текущем месяце
/// `current`: для отклонения расходов от среднего за год читаются и месяцы до `current` того же
/// года (годовое среднее считается по месяцам не позже текущего).
///
/// # Errors
/// Месяц вне диапазона `YearMonth`.
pub fn overview_last_month(month: YearMonth, current: YearMonth) -> Result<YearMonth, CoreError> {
    Ok(if current.year() > month.year() {
        YearMonth::new(month.year(), 12)?
    } else if current.year() == month.year() && current > month {
        current
    } else {
        month
    })
}

/// Границы ряда месяцев (первый и последний) для выбранного месяца.
///
/// # Errors
/// Границы вне диапазона `YearMonth`.
pub fn series_bounds(
    month: YearMonth,
    range: SeriesRange,
) -> Result<(YearMonth, YearMonth), CoreError> {
    let year = month.year();
    Ok(match range {
        SeriesRange::Year => (YearMonth::first_of_year(year)?, YearMonth::new(year, 12)?),
        SeriesRange::Last12 => ((0..11).fold(month, |m, _| m.pred().unwrap_or(m)), month),
        SeriesRange::All => (
            YearMonth::first_of_year(year.saturating_sub(MAX_YEARS_BACK).max(1970))?,
            YearMonth::new(year, 12)?,
        ),
    })
}

/// Входит ли месяц в ряд: в периоде «Всё» месяцы без дохода и расходов пропускаются.
#[must_use]
pub fn series_keeps(range: SeriesRange, summary: &MonthSummary) -> bool {
    range != SeriesRange::All || !(summary.income.is_zero() && summary.expenses.is_zero())
}

#[cfg(test)]
mod series_tests {
    use super::*;

    fn ym(s: &str) -> YearMonth {
        YearMonth::parse(s).unwrap()
    }

    #[test]
    fn overview_reads_up_to_the_current_month_of_the_same_year() {
        // Прошлый год читается до декабря, текущий — до сегодняшнего месяца, будущий — до себя.
        assert_eq!(
            overview_last_month(ym("2025-03"), ym("2026-10")),
            Ok(ym("2025-12"))
        );
        assert_eq!(
            overview_last_month(ym("2026-03"), ym("2026-10")),
            Ok(ym("2026-10"))
        );
        assert_eq!(
            overview_last_month(ym("2026-11"), ym("2026-10")),
            Ok(ym("2026-11"))
        );
        assert_eq!(
            overview_last_month(ym("2027-01"), ym("2026-10")),
            Ok(ym("2027-01"))
        );
    }

    #[test]
    fn series_bounds_follow_the_range() {
        let m = ym("2026-10");
        assert_eq!(
            series_bounds(m, SeriesRange::Year),
            Ok((ym("2026-01"), ym("2026-12")))
        );
        assert_eq!(
            series_bounds(m, SeriesRange::Last12),
            Ok((ym("2025-11"), m))
        );
        assert_eq!(
            series_bounds(m, SeriesRange::All),
            Ok((ym("2016-01"), ym("2026-12")))
        );
        // Не раньше 1970.
        assert_eq!(
            series_bounds(ym("1972-05"), SeriesRange::All),
            Ok((ym("1970-01"), ym("1972-12")))
        );
    }
}

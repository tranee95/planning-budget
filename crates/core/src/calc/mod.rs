//! Расчёты. Один проход по `DataSet` строит `Ledger`; все сводки читают из него.

mod bonds;
mod debts;
mod limits;
mod plan;
mod savings;
mod summary;

use std::collections::BTreeMap;

pub use bonds::{BondRates, BondsFact, BondsForecast, BondsMonth, ForecastScenario, Scenario};
pub use debts::{
    DebtState, DebtsSummary, debt_progress, equal_parts_schedule, single_payment_schedule,
    validate_schedule,
};
pub(crate) use limits::usage;
pub use limits::{BudgetBalance, CategoryYearRow, LimitLevel, LimitRow, MonthLimits};
pub use plan::{LockSnapshot, MonthPlan, PlanBalance, PlanCopyRow, PlanRow};
pub use savings::{Accumulation, SavingsOverview};
pub use summary::{Corridor, MonthSummary, YearSummary};

use crate::error::{CoreError, MoneyError};
use crate::model::{
    BasisPoints, CategoryId, CategoryKind, DataSet, DebtPaymentStatus, IncomeStatus, TxStatus,
};
use crate::money::Money;
use crate::period::YearMonth;

/// Доли статусов в сумме по всем статусам, базисные пункты (10 000 = 100 %).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StatusShares {
    pub paid: i32,
    pub debt: i32,
    pub unplanned: i32,
    pub planned: i32,
}

/// Суммы по четырём статусам трат.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StatusAmounts {
    pub paid: Money,
    pub debt: Money,
    pub unplanned: Money,
    pub planned: Money,
}

impl StatusAmounts {
    pub fn get(&self, status: TxStatus) -> Money {
        match status {
            TxStatus::Paid => self.paid,
            TxStatus::Debt => self.debt,
            TxStatus::Unplanned => self.unplanned,
            TxStatus::Planned => self.planned,
        }
    }

    fn slot(&mut self, status: TxStatus) -> &mut Money {
        match status {
            TxStatus::Paid => &mut self.paid,
            TxStatus::Debt => &mut self.debt,
            TxStatus::Unplanned => &mut self.unplanned,
            TxStatus::Planned => &mut self.planned,
        }
    }

    pub(crate) fn add(&mut self, status: TxStatus, amount: Money) -> Result<(), MoneyError> {
        let slot = self.slot(status);
        *slot = slot.checked_add(amount)?;
        Ok(())
    }

    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), MoneyError> {
        for status in [
            TxStatus::Paid,
            TxStatus::Debt,
            TxStatus::Unplanned,
            TxStatus::Planned,
        ] {
            self.add(status, other.get(status))?;
        }
        Ok(())
    }

    /// Сумма по всем статусам.
    pub fn total(&self) -> Result<Money, MoneyError> {
        Money::sum([self.paid, self.debt, self.unplanned, self.planned])
    }

    /// Доли статусов в общей сумме; при нулевой (или не помещающейся в `Money`) сумме все доли 0.
    pub fn shares_bp(&self) -> StatusShares {
        let total = self.total().unwrap_or(Money::ZERO);
        let share = |amount: Money| amount.ratio_bp(total).unwrap_or(0);
        StatusShares {
            paid: share(self.paid),
            debt: share(self.debt),
            unplanned: share(self.unplanned),
            planned: share(self.planned),
        }
    }
}

/// Суммы по четырём типам категорий.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct KindAmounts {
    pub mandatory: Money,
    pub wants: Money,
    pub savings: Money,
    pub loans: Money,
}

impl KindAmounts {
    pub fn get(&self, kind: CategoryKind) -> Money {
        match kind {
            CategoryKind::Mandatory => self.mandatory,
            CategoryKind::Wants => self.wants,
            CategoryKind::Savings => self.savings,
            CategoryKind::Loans => self.loans,
        }
    }

    fn slot(&mut self, kind: CategoryKind) -> &mut Money {
        match kind {
            CategoryKind::Mandatory => &mut self.mandatory,
            CategoryKind::Wants => &mut self.wants,
            CategoryKind::Savings => &mut self.savings,
            CategoryKind::Loans => &mut self.loans,
        }
    }

    pub(crate) fn add(&mut self, kind: CategoryKind, amount: Money) -> Result<(), MoneyError> {
        let slot = self.slot(kind);
        *slot = slot.checked_add(amount)?;
        Ok(())
    }

    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), MoneyError> {
        for kind in [
            CategoryKind::Mandatory,
            CategoryKind::Wants,
            CategoryKind::Savings,
            CategoryKind::Loans,
        ] {
            self.add(kind, other.get(kind))?;
        }
        Ok(())
    }

    /// Всё, кроме сбережений.
    pub fn expenses(&self) -> Result<Money, MoneyError> {
        Money::sum([self.mandatory, self.wants, self.loans])
    }
}

/// Агрегаты одного месяца.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct MonthTotals {
    pub income_received: Money,
    pub income_expected: Money,
    pub by_status: StatusAmounts,
    pub by_kind: KindAmounts,
    /// Долги, взятые в месяце: источник денег.
    pub borrowed: Money,
    /// Оплаченные погашения месяца: выплата, не расход.
    pub repaid: Money,
}

impl MonthTotals {
    /// Добавляет суммы трат серии; доходы месяца считаются отдельным проходом.
    fn merge_expenses(&mut self, other: &Self) -> Result<(), MoneyError> {
        self.by_status.merge(&other.by_status)?;
        self.by_kind.merge(&other.by_kind)
    }

    pub fn income(&self) -> Result<Money, MoneyError> {
        self.income_received.checked_add(self.income_expected)
    }

    /// «Свободный остаток»: `income − expenses − savings + borrowed − repaid`.
    pub fn free(&self) -> Result<Money, MoneyError> {
        self.income()?
            .checked_sub(self.by_kind.expenses()?)?
            .checked_sub(self.by_kind.savings)?
            .checked_add(self.borrowed)?
            .checked_sub(self.repaid)
    }
}

/// Предвычисленные агрегаты над `DataSet`: суммы по месяцам, категориям и статусам.
#[derive(Debug)]
pub struct Ledger<'a> {
    data: &'a DataSet,
    kinds: BTreeMap<CategoryId, CategoryKind>,
    by_category: BTreeMap<(YearMonth, CategoryId), StatusAmounts>,
    months: BTreeMap<YearMonth, MonthTotals>,
    limit_history: BTreeMap<CategoryId, BTreeMap<YearMonth, Money>>,
    savings_rate_history: BTreeMap<CategoryId, BTreeMap<YearMonth, PlanEntry>>,
}

/// Строка истории плана накопления: процент либо фиксированная сумма.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PlanEntry {
    pub rate: BasisPoints,
    pub fixed: Option<Money>,
}

impl<'a> Ledger<'a> {
    /// Один проход по тратам и доходам. Трата с неизвестной категорией — ошибка.
    pub fn new(data: &'a DataSet) -> Result<Self, CoreError> {
        if data.settings.weeks_per_month == 0 {
            return Err(CoreError::Settings("weeks_per_month must be positive"));
        }
        let kinds: BTreeMap<_, _> = data.categories.iter().map(|c| (c.id, c.kind)).collect();

        let mut by_category: BTreeMap<(YearMonth, CategoryId), StatusAmounts> = BTreeMap::new();
        let mut months: BTreeMap<YearMonth, MonthTotals> = BTreeMap::new();
        // Репозитории отдают траты упорядоченными по (месяц, категория): подряд идущие записи
        // накапливаются в серию и попадают в карты только при смене ключа, а не поиском на каждую.
        let mut kind_cache: Option<(CategoryId, CategoryKind)> = None;
        let mut category_run: Option<((YearMonth, CategoryId), StatusAmounts)> = None;
        let mut month_run: Option<(YearMonth, MonthTotals)> = None;
        for tx in &data.transactions {
            let kind = match kind_cache {
                Some((id, kind)) if id == tx.category_id => kind,
                _ => {
                    let kind = *kinds
                        .get(&tx.category_id)
                        .ok_or(CoreError::CategoryNotFound(tx.category_id.0))?;
                    kind_cache = Some((tx.category_id, kind));
                    kind
                }
            };
            let key = (tx.month, tx.category_id);
            match &mut category_run {
                Some((run_key, amounts)) if *run_key == key => amounts.add(tx.status, tx.amount)?,
                run => {
                    if let Some((done_key, done)) = run.take() {
                        by_category.entry(done_key).or_default().merge(&done)?;
                    }
                    let mut amounts = StatusAmounts::default();
                    amounts.add(tx.status, tx.amount)?;
                    *run = Some((key, amounts));
                }
            }
            match &mut month_run {
                Some((run_month, totals)) if *run_month == tx.month => {
                    totals.by_status.add(tx.status, tx.amount)?;
                    totals.by_kind.add(kind, tx.amount)?;
                }
                run => {
                    if let Some((done_month, done)) = run.take() {
                        months
                            .entry(done_month)
                            .or_default()
                            .merge_expenses(&done)?;
                    }
                    let mut totals = MonthTotals::default();
                    totals.by_status.add(tx.status, tx.amount)?;
                    totals.by_kind.add(kind, tx.amount)?;
                    *run = Some((tx.month, totals));
                }
            }
        }
        if let Some((key, done)) = category_run {
            by_category.entry(key).or_default().merge(&done)?;
        }
        if let Some((month, done)) = month_run {
            months.entry(month).or_default().merge_expenses(&done)?;
        }
        for income in &data.incomes {
            let totals = months.entry(income.month).or_default();
            let slot = match income.status {
                IncomeStatus::Received => &mut totals.income_received,
                IncomeStatus::Expected => &mut totals.income_expected,
            };
            *slot = slot.checked_add(income.amount)?;
        }

        for debt in &data.debts {
            let totals = months.entry(debt.taken_month).or_default();
            totals.borrowed = totals.borrowed.checked_add(debt.amount)?;
        }
        for payment in &data.debt_payments {
            if payment.status == DebtPaymentStatus::Paid {
                let totals = months.entry(payment.month).or_default();
                totals.repaid = totals.repaid.checked_add(payment.amount)?;
            }
        }

        let mut limit_history: BTreeMap<CategoryId, BTreeMap<YearMonth, Money>> = BTreeMap::new();
        for entry in &data.limits {
            limit_history
                .entry(entry.category_id)
                .or_default()
                .insert(entry.valid_from, entry.amount);
        }

        let mut savings_rate_history: BTreeMap<CategoryId, BTreeMap<YearMonth, PlanEntry>> =
            BTreeMap::new();
        for entry in &data.savings_rates {
            savings_rate_history
                .entry(entry.category_id)
                .or_default()
                .insert(
                    entry.valid_from,
                    PlanEntry {
                        rate: entry.rate,
                        fixed: entry.fixed_amount,
                    },
                );
        }

        Ok(Self {
            data,
            kinds,
            by_category,
            months,
            limit_history,
            savings_rate_history,
        })
    }

    pub fn data(&self) -> &'a DataSet {
        self.data
    }

    pub(crate) fn totals(&self, month: YearMonth) -> MonthTotals {
        self.months.get(&month).copied().unwrap_or_default()
    }

    /// Суммы категории за месяц по статусам.
    pub fn category_by_status(&self, month: YearMonth, category: CategoryId) -> StatusAmounts {
        self.by_category
            .get(&(month, category))
            .copied()
            .unwrap_or_default()
    }

    /// Сумма трат категории за месяц, все статусы.
    pub fn category_total(
        &self,
        month: YearMonth,
        category: CategoryId,
    ) -> Result<Money, MoneyError> {
        self.category_by_status(month, category).total()
    }

    /// Сумма трат категории за месяцы года не позже `current`.
    pub fn category_year_total(
        &self,
        year: u16,
        current: YearMonth,
        category: CategoryId,
    ) -> Result<Money, CoreError> {
        let mut total = Money::ZERO;
        for month in YearMonth::months_of_year(year)?.filter(|m| *m <= current) {
            total = total.checked_add(self.category_total(month, category)?)?;
        }
        Ok(total)
    }

    pub(crate) fn kind_of(&self, category: CategoryId) -> Option<CategoryKind> {
        self.kinds.get(&category).copied()
    }
}

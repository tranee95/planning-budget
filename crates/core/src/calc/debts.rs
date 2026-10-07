//! Долги: остаток, график погашений и его проверка.

use super::{Ledger, MoneyError};
use crate::error::CoreError;
use crate::model::{DebtId, DebtPayment, DebtPaymentStatus};
use crate::money::Money;
use crate::period::YearMonth;

/// Сколько месяцев максимум занимает график: больше — почти наверняка ошибка ввода.
const MAX_SCHEDULE_MONTHS: u32 = 600;

/// График «равными частями на `months` месяцев», начиная с `first_month`. Остаток копеек уходит
/// в последний платёж; каждая часть положительна, поэтому `amount` не меньше `months` копеек.
pub fn equal_parts_schedule(
    amount: Money,
    first_month: YearMonth,
    months: u32,
) -> Result<Vec<(YearMonth, Money)>, CoreError> {
    if months == 0 || months > MAX_SCHEDULE_MONTHS {
        return Err(CoreError::Schedule("months out of range"));
    }
    if amount <= Money::ZERO {
        return Err(CoreError::Schedule("amount must be positive"));
    }
    let n = i64::from(months);
    let part = Money::from_kopecks(
        amount
            .kopecks()
            .checked_div(n)
            .ok_or(MoneyError::DivisionByZero)?,
    );
    if part <= Money::ZERO {
        return Err(CoreError::Schedule(
            "amount is smaller than the number of months",
        ));
    }
    let mut rows = Vec::with_capacity(usize::try_from(months).unwrap_or(0));
    let mut month = first_month;
    let mut left = amount;
    for i in 0..months {
        let value = if i + 1 == months { left } else { part };
        rows.push((month, value));
        left = left.checked_sub(value)?;
        if i + 1 < months {
            month = month.succ().ok_or(CoreError::Month)?;
        }
    }
    Ok(rows)
}

/// График «одним платежом в месяце `month`».
pub fn single_payment_schedule(
    amount: Money,
    taken_month: YearMonth,
    month: YearMonth,
) -> Result<Vec<(YearMonth, Money)>, CoreError> {
    if amount <= Money::ZERO {
        return Err(CoreError::Schedule("amount must be positive"));
    }
    if month < taken_month {
        return Err(CoreError::Schedule("payment before the month of the loan"));
    }
    Ok(vec![(month, amount)])
}

/// Проверка графика: суммы положительны, месяцы уникальны и не раньше месяца займа, сумма равна
/// сумме долга.
pub fn validate_schedule(
    amount: Money,
    taken_month: YearMonth,
    payments: &[(YearMonth, Money)],
) -> Result<(), CoreError> {
    if payments.is_empty() {
        return Err(CoreError::Schedule("schedule is empty"));
    }
    let mut total = Money::ZERO;
    let mut seen = std::collections::BTreeSet::new();
    for (month, value) in payments {
        if *value <= Money::ZERO {
            return Err(CoreError::Schedule("payment must be positive"));
        }
        if *month < taken_month {
            return Err(CoreError::Schedule("payment before the month of the loan"));
        }
        if !seen.insert(*month) {
            return Err(CoreError::Schedule("duplicate month"));
        }
        total = total.checked_add(*value)?;
    }
    if total != amount {
        return Err(CoreError::Schedule(
            "schedule does not add up to the debt amount",
        ));
    }
    Ok(())
}

/// Остаток долга и ближайший платёж.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DebtState {
    /// `amount − Σ оплаченных`.
    pub remaining: Money,
    /// Первый по месяцу неоплаченный платёж.
    pub next_payment: Option<DebtPayment>,
    /// Все платежи оплачены (остаток 0).
    pub closed: bool,
}

/// Итоги раздела «Долги» на месяц.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DebtsSummary {
    /// Долги с ненулевым остатком.
    pub open_count: u32,
    /// Полностью погашенные долги.
    pub closed_count: u32,
    /// Сумма остатков открытых долгов.
    pub remaining_total: Money,
    /// Все строки графиков месяца (любой статус): то, что нужно выплатить за месяц.
    pub payments_planned: Money,
    /// Из них оплачено.
    pub payments_paid: Money,
}

/// Остаток долга и ближайший платёж по строкам его графика.
pub fn debt_progress<'a>(
    amount: Money,
    payments: impl IntoIterator<Item = &'a DebtPayment>,
) -> Result<DebtState, CoreError> {
    let mut paid = Money::ZERO;
    let mut next: Option<DebtPayment> = None;
    for p in payments {
        match p.status {
            DebtPaymentStatus::Paid => paid = paid.checked_add(p.amount)?,
            DebtPaymentStatus::Planned => {
                if next.is_none_or(|n| p.month < n.month) {
                    next = Some(*p);
                }
            }
        }
    }
    let remaining = amount.checked_sub(paid)?;
    Ok(DebtState {
        remaining,
        next_payment: next,
        closed: remaining.is_zero(),
    })
}

impl Ledger<'_> {
    /// Итоги по долгам: остаток, платежи месяца, число открытых и закрытых.
    pub fn debts_summary(&self, month: YearMonth) -> Result<DebtsSummary, CoreError> {
        let (mut open, mut closed) = (0_u32, 0_u32);
        let mut remaining_total = Money::ZERO;
        for debt in &self.data().debts {
            let Some(state) = self.debt_state(debt.id)? else {
                continue;
            };
            if state.closed {
                closed = closed.saturating_add(1);
            } else {
                open = open.saturating_add(1);
                remaining_total = remaining_total.checked_add(state.remaining)?;
            }
        }
        Ok(DebtsSummary {
            open_count: open,
            closed_count: closed,
            remaining_total,
            payments_planned: self.repayments_planned(month)?,
            payments_paid: self.totals(month).repaid,
        })
    }

    /// Состояние долга по данным набора; `None`, если долга с таким id нет.
    pub fn debt_state(&self, id: DebtId) -> Result<Option<DebtState>, CoreError> {
        let Some(debt) = self.data().debts.iter().find(|d| d.id == id) else {
            return Ok(None);
        };
        let payments = self.data().debt_payments.iter().filter(|p| p.debt_id == id);
        Ok(Some(debt_progress(debt.amount, payments)?))
    }

    /// План погашений месяца: все строки графиков месяца (любой статус).
    pub fn repayments_planned(&self, month: YearMonth) -> Result<Money, CoreError> {
        self.data()
            .debt_payments
            .iter()
            .filter(|p| p.month == month)
            .try_fold(Money::ZERO, |sum, p| Ok(sum.checked_add(p.amount)?))
    }
}

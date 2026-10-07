//! План месяца: «Не распределено», план → факт, фиксация и копирование.

use super::Ledger;
use crate::error::CoreError;
use crate::model::{CategoryId, CategoryKind, TxId, TxStatus};
use crate::money::Money;
use crate::period::YearMonth;

/// Строка «план → факт» по статье или накоплению.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlanRow {
    pub category_id: CategoryId,
    pub plan: Money,
    /// Все траты категории за месяц, любой статус.
    pub fact: Money,
    /// `fact − plan`: положительное — потрачено больше плана.
    pub deviation: Money,
}

/// План месяца.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MonthPlan {
    pub month: YearMonth,
    /// Месяц зафиксирован кнопкой «План готов».
    pub locked: bool,
    pub income: Money,
    pub plan_expenses: Money,
    pub plan_savings: Money,
    pub plan_repayments: Money,
    /// `income − plan_expenses − plan_savings − plan_repayments`: цель 0, отрицательное — план превышает доход.
    pub unallocated: Money,
    /// Строки по категориям: статьи и накопления, у которых есть план или факт.
    pub rows: Vec<PlanRow>,
    /// Траты со статусом «Незапланировано».
    pub unplanned: Money,
    /// Взято в долг в этом месяце.
    pub borrowed: Money,
    /// Отложено (факт по накоплениям).
    pub saved: Money,
}

/// Итог распределения дохода: что показать рядом с «Не распределено».
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlanBalance {
    /// Дохода нет и ничего не запланировано.
    Empty,
    /// Каждый рубль дохода получил назначение: `unallocated = 0`.
    Balanced,
    /// Часть дохода без назначения: `unallocated > 0`.
    Unallocated,
    /// План превышает доход: `unallocated < 0`.
    Over,
}

impl MonthPlan {
    /// Итог распределения дохода.
    pub fn balance(&self) -> PlanBalance {
        let planned = [self.plan_expenses, self.plan_savings, self.plan_repayments];
        if self.income.is_zero() && planned.iter().all(|m| m.is_zero()) {
            PlanBalance::Empty
        } else if self.unallocated.is_zero() {
            PlanBalance::Balanced
        } else if self.unallocated > Money::ZERO {
            PlanBalance::Unallocated
        } else {
            PlanBalance::Over
        }
    }
}

/// Что записать при фиксации плана: решение принимает ядро, запись — хранилище.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LockSnapshot {
    /// Плановая сумма трат: `planned_amount = amount` для статусов «План», «Оплачено», «Долг».
    pub planned_amounts: Vec<(TxId, Money)>,
    /// План по накоплениям на момент фиксации.
    pub savings: Vec<(CategoryId, Money)>,
    pub repayments_planned: Money,
}

/// Строка плана прошлого месяца, которую «Скопировать план» создаёт заново.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlanCopyRow {
    pub category_id: CategoryId,
    pub title: String,
    pub amount: Money,
}

impl Ledger<'_> {
    fn is_savings_category(&self, category: CategoryId) -> bool {
        self.kind_of(category) == Some(CategoryKind::Savings)
    }

    /// Плановая сумма траты в незафиксированном месяце: «Незапланировано» в план не входит.
    fn open_plan_amount(status: TxStatus, amount: Money) -> Option<Money> {
        match status {
            TxStatus::Planned | TxStatus::Paid | TxStatus::Debt => Some(amount),
            TxStatus::Unplanned => None,
        }
    }

    /// План месяца с «Не распределено» и сравнением план → факт.
    pub fn month_plan(&self, month: YearMonth) -> Result<MonthPlan, CoreError> {
        let locked = self.data().locked_plans.get(&month);
        let totals = self.totals(month);
        let income = totals.income()?;

        let mut plan_by_category: std::collections::BTreeMap<CategoryId, Money> =
            std::collections::BTreeMap::new();
        for tx in self.data().transactions.iter().filter(|t| t.month == month) {
            let planned = if locked.is_some() {
                tx.planned_amount
            } else {
                Self::open_plan_amount(tx.status, tx.amount)
            };
            if let Some(amount) = planned {
                let slot = plan_by_category
                    .entry(tx.category_id)
                    .or_insert(Money::ZERO);
                *slot = slot.checked_add(amount)?;
            }
        }

        let mut plan_expenses = Money::ZERO;
        let mut plan_savings = Money::ZERO;
        let mut rows = Vec::new();
        for category in &self.data().categories {
            let is_savings = category.kind == CategoryKind::Savings;
            let plan = if is_savings {
                match locked {
                    Some(l) => l.savings.get(&category.id).copied().unwrap_or(Money::ZERO),
                    None => self.category_savings_plan(month, category.id)?,
                }
            } else {
                plan_by_category
                    .get(&category.id)
                    .copied()
                    .unwrap_or(Money::ZERO)
            };
            let fact = self.category_total(month, category.id)?;
            if is_savings {
                plan_savings = plan_savings.checked_add(plan)?;
            } else {
                plan_expenses = plan_expenses.checked_add(plan)?;
            }
            if category.archived && plan.is_zero() && fact.is_zero() {
                continue;
            }
            rows.push(PlanRow {
                category_id: category.id,
                plan,
                fact,
                deviation: fact.checked_sub(plan)?,
            });
        }
        let plan_repayments = match locked {
            Some(l) => l.repayments_planned,
            None => self.repayments_planned(month)?,
        };
        let unallocated = income
            .checked_sub(plan_expenses)?
            .checked_sub(plan_savings)?
            .checked_sub(plan_repayments)?;
        Ok(MonthPlan {
            month,
            locked: locked.is_some(),
            income,
            plan_expenses,
            plan_savings,
            plan_repayments,
            unallocated,
            rows,
            unplanned: totals.by_status.unplanned,
            borrowed: totals.borrowed,
            saved: totals.by_kind.savings,
        })
    }

    /// Что записать при «План готов» (повторная фиксация после разблокировки считается заново,
    /// поэтому существующая блокировка месяца игнорируется).
    pub fn lock_snapshot(&self, month: YearMonth) -> Result<LockSnapshot, CoreError> {
        let planned_amounts = self
            .data()
            .transactions
            .iter()
            .filter(|t| t.month == month)
            .filter_map(|t| Self::open_plan_amount(t.status, t.amount).map(|a| (t.id, a)))
            .collect();
        let mut savings = Vec::new();
        for category in self
            .data()
            .categories
            .iter()
            .filter(|c| c.kind == CategoryKind::Savings)
        {
            savings.push((category.id, self.category_savings_plan(month, category.id)?));
        }
        Ok(LockSnapshot {
            planned_amounts,
            savings,
            repayments_planned: self.repayments_planned(month)?,
        })
    }

    /// Строки, которые «Скопировать план из прошлого месяца» создаёт в целевом месяце со статусом «План»:
    /// плановые суммы источника (по фиксации, если месяц зафиксирован, иначе по статусам), без
    /// накоплений: их план идёт по процентам и суммам, а погашения — по графикам долгов.
    pub fn plan_copy_rows(&self, from: YearMonth) -> Vec<PlanCopyRow> {
        let locked = self.data().locked_plans.contains_key(&from);
        self.data()
            .transactions
            .iter()
            .filter(|t| t.month == from && !self.is_savings_category(t.category_id))
            .filter_map(|t| {
                let amount = if locked {
                    t.planned_amount
                } else {
                    Self::open_plan_amount(t.status, t.amount)
                }?;
                Some(PlanCopyRow {
                    category_id: t.category_id,
                    title: t.title.clone(),
                    amount,
                })
            })
            .collect()
    }
}

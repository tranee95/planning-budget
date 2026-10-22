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
    /// Записи категории без статуса «План»: оплачено, долг и внеплановые.
    pub actual: Money,
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
    /// Отложено: записи накоплений без статуса «План», как на экране «Сбережения».
    pub saved: Money,
    /// В месяце есть записи со статусом «План»: мастер плана заполняет только месяц без них.
    pub has_planned_lines: bool,
    /// Фиксированный план накоплений, действующий в месяце (строка истории с суммой, а не
    /// процентом); мастер плана подставляет его в поля накоплений.
    pub savings_fixed: Vec<SavingsFixed>,
}

/// Фиксированная ежемесячная сумма накопления, действующая в месяце.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SavingsFixed {
    pub category_id: CategoryId,
    pub fixed: Money,
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
        let mut savings_fixed = Vec::new();
        for category in &self.data().categories {
            let is_savings = category.kind == CategoryKind::Savings;
            if let Some(fixed) = is_savings
                .then(|| self.plan_entry(month, category.id).and_then(|e| e.fixed))
                .flatten()
            {
                savings_fixed.push(SavingsFixed {
                    category_id: category.id,
                    fixed,
                });
            }
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
            let by_status = self.category_by_status(month, category.id);
            let fact = by_status.total()?;
            let actual = fact.checked_sub(by_status.planned)?;
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
                actual,
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
            saved: totals.savings_actual()?,
            has_planned_lines: self
                .data()
                .transactions
                .iter()
                .any(|t| t.month == month && t.status == TxStatus::Planned),
            savings_fixed,
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

/// Что известно о месяце, в который переносят план (, «Автоперенос (U-1)»).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CarryFacts {
    /// Месяц, в который переносят.
    pub to: YearMonth,
    /// Текущий месяц по местной дате пользователя.
    pub today: YearMonth,
    /// Месяц зафиксирован кнопкой «План готов».
    pub locked: bool,
    /// Автоперенос в этот месяц уже выполнялся (отметка `month_carry`).
    pub already_carried: bool,
    /// В месяце есть записи.
    pub has_records: bool,
    /// Данные есть в более поздних месяцах: «последнего» месяца для `to` нет.
    pub newer_data: bool,
    /// Ближайший прошлый месяц с тратами не в накоплениях.
    pub source: Option<YearMonth>,
}

/// Решение автопереноса.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CarryDecision {
    /// Ничего не делать и отметку не ставить.
    Skip,
    /// Скопировать плановые строки из месяца и поставить отметку.
    CopyFrom(YearMonth),
    /// Копировать нечего (в месяце уже есть записи): только поставить отметку.
    MarkOnly,
}

/// Правила автопереноса: не позже текущего месяца, не зафиксированный, не переносившийся, без
/// более новых данных и с источником; месяц с записями только получает отметку.
#[must_use]
pub fn carry_decision(facts: &CarryFacts) -> CarryDecision {
    if facts.to > facts.today || facts.locked || facts.already_carried || facts.newer_data {
        return CarryDecision::Skip;
    }
    match facts.source {
        Some(from) if !facts.has_records => CarryDecision::CopyFrom(from),
        Some(_) => CarryDecision::MarkOnly,
        None => CarryDecision::Skip,
    }
}

#[cfg(test)]
mod carry_tests {
    use super::*;

    fn ym(s: &str) -> YearMonth {
        YearMonth::parse(s).unwrap()
    }

    fn open() -> CarryFacts {
        CarryFacts {
            to: ym("2026-11"),
            today: ym("2026-11"),
            locked: false,
            already_carried: false,
            has_records: false,
            newer_data: false,
            source: Some(ym("2026-10")),
        }
    }

    #[test]
    fn copies_from_the_source_into_an_empty_current_month() {
        assert_eq!(
            carry_decision(&open()),
            CarryDecision::CopyFrom(ym("2026-10"))
        );
    }

    #[test]
    fn a_month_with_records_is_only_marked() {
        let facts = CarryFacts {
            has_records: true,
            ..open()
        };
        assert_eq!(carry_decision(&facts), CarryDecision::MarkOnly);
    }

    #[test]
    fn skips_future_locked_carried_and_older_than_data() {
        for facts in [
            CarryFacts {
                today: ym("2026-10"),
                ..open()
            },
            CarryFacts {
                locked: true,
                ..open()
            },
            CarryFacts {
                already_carried: true,
                ..open()
            },
            CarryFacts {
                newer_data: true,
                ..open()
            },
            CarryFacts {
                source: None,
                ..open()
            },
        ] {
            assert_eq!(carry_decision(&facts), CarryDecision::Skip, "{facts:?}");
        }
    }
}

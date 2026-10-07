//! План месяца поверх `core::calc` и хранилища.

use chrono::{DateTime, Utc};
use planning_budget_core::calc::Ledger;
use planning_budget_core::{
    BasisPoints, CategoryId, CategoryKind, Income, IncomeId, IncomeStatus, Money, SavingsRateEntry,
    Transaction, TxId, TxStatus, YearMonth,
};
use planning_budget_storage::{Db, PlanWizardInput, SavingsPlanInput};

use super::parse_month;
use crate::AppError;
use crate::dto::{MonthPlanDto, PlanRepaymentDto, PlanWizardInputDto, WizardSavingsPlanDto};

fn load(db: &Db, month: YearMonth) -> Result<MonthPlanDto, AppError> {
    let data = db.dataset(month, month)?;
    plan_of(db, &data, month)
}

/// План месяца по набору данных: погашения долгов добавляются из хранилища.
fn plan_of(
    db: &Db,
    data: &planning_budget_core::DataSet,
    month: YearMonth,
) -> Result<MonthPlanDto, AppError> {
    let plan = Ledger::new(data)?.month_plan(month)?;
    let mut dto = MonthPlanDto::from(&plan);
    for debt in db.debts(true)? {
        for payment in debt.payments.iter().filter(|p| p.month == month) {
            dto.repayments.push(PlanRepaymentDto {
                payment_id: payment.id,
                debt_id: debt.id.0,
                lender: debt.lender.clone(),
                amount: payment.amount.kopecks(),
                status: payment.status.into(),
            });
        }
    }
    Ok(dto)
}

/// План месяца: «Не распределено», план → факт по статьям.
pub fn month(db: &Db, month: &str) -> Result<MonthPlanDto, AppError> {
    load(db, parse_month(month, "month")?)
}

/// «План готов»: фиксирует плановые суммы месяца.
pub fn lock(db: &mut Db, month: &str, now: DateTime<Utc>) -> Result<MonthPlanDto, AppError> {
    let month = parse_month(month, "month")?;
    db.plan_lock(month, now)?;
    load(db, month)
}

/// Снимает фиксацию: план снова читается из текущих сумм.
pub fn unlock(db: &mut Db, month: &str) -> Result<MonthPlanDto, AppError> {
    let month = parse_month(month, "month")?;
    db.plan_unlock(month)?;
    load(db, month)
}

/// «Скопировать план из прошлого месяца» в `month`.
pub fn copy_from_previous(
    db: &mut Db,
    month: &str,
    now: DateTime<Utc>,
) -> Result<MonthPlanDto, AppError> {
    let to = parse_month(month, "month")?;
    let from = to
        .pred()
        .ok_or_else(|| AppError::invalid_field("month_invalid", "month"))?;
    db.plan_copy_from(from, to, now)?;
    load(db, to)
}

fn wizard_input(input: &PlanWizardInputDto) -> PlanWizardInput {
    PlanWizardInput {
        incomes: input
            .incomes
            .iter()
            .map(|i| (i.source_name.clone(), Money::from_kopecks(i.amount)))
            .collect(),
        lines: input
            .lines
            .iter()
            .map(|l| (CategoryId(l.category_id), Money::from_kopecks(l.amount)))
            .collect(),
        savings: input
            .savings
            .iter()
            .map(|s| {
                let plan = match &s.plan {
                    WizardSavingsPlanDto::Percent { rate_bp } => {
                        SavingsPlanInput::Percent(BasisPoints(*rate_bp))
                    }
                    WizardSavingsPlanDto::Fixed { amount } => {
                        SavingsPlanInput::Fixed(Money::from_kopecks(*amount))
                    }
                };
                (CategoryId(s.category_id), plan)
            })
            .collect(),
    }
}

/// Предпросмотр мастера: «Не распределено» по введённому, без записи. Расчёт ядра на копии набора
/// данных месяца; неполные строки (нулевая сумма, неизвестная статья) пропускаются.
pub fn preview(db: &Db, month: &str, input: &PlanWizardInputDto) -> Result<MonthPlanDto, AppError> {
    let month = parse_month(month, "month")?;
    let mut data = db.dataset(month, month)?;
    for (i, income) in input
        .incomes
        .iter()
        .enumerate()
        .filter(|(_, i)| i.amount > 0)
    {
        data.incomes.push(Income {
            id: IncomeId(-1 - i64::try_from(i).unwrap_or(0)),
            month,
            source_name: income.source_name.clone(),
            amount: Money::from_kopecks(income.amount),
            status: IncomeStatus::Expected,
        });
    }
    for (i, line) in input.lines.iter().enumerate().filter(|(_, l)| l.amount > 0) {
        let known = data
            .categories
            .iter()
            .any(|c| c.id.0 == line.category_id && c.kind != CategoryKind::Savings);
        if !known {
            continue;
        }
        data.transactions.push(Transaction {
            id: TxId(-1 - i64::try_from(i).unwrap_or(0)),
            month,
            category_id: CategoryId(line.category_id),
            title: String::new(),
            amount: Money::from_kopecks(line.amount),
            status: TxStatus::Planned,
            planned_amount: None,
        });
    }
    for saving in &input.savings {
        let category = CategoryId(saving.category_id);
        let is_savings = data
            .categories
            .iter()
            .any(|c| c.id == category && c.kind == CategoryKind::Savings);
        if !is_savings {
            continue;
        }
        data.savings_overrides.remove(&(month, category));
        data.savings_rates
            .retain(|r| !(r.category_id == category && r.valid_from == month));
        let (rate, fixed_amount) = match &saving.plan {
            WizardSavingsPlanDto::Percent { rate_bp } => (BasisPoints(*rate_bp), None),
            WizardSavingsPlanDto::Fixed { amount } => {
                (BasisPoints(0), Some(Money::from_kopecks(*amount)))
            }
        };
        data.savings_rates.push(SavingsRateEntry {
            category_id: category,
            valid_from: month,
            rate,
            fixed_amount,
        });
    }
    plan_of(db, &data, month)
}

/// Мастер первого месяца: доходы, плановые траты и план накоплений одной транзакцией, затем
/// «План готов». Возвращает зафиксированный план.
pub fn wizard_apply(
    db: &mut Db,
    month: &str,
    input: &PlanWizardInputDto,
    now: DateTime<Utc>,
) -> Result<MonthPlanDto, AppError> {
    let month = parse_month(month, "month")?;
    db.plan_wizard_apply(month, &wizard_input(input), now)?;
    load(db, month)
}

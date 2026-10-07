//! План месяца: «Не распределено», план → факт.

use planning_budget_core::calc::{MonthPlan, PlanBalance, PlanRow};
use serde::{Deserialize, Serialize};

use super::DebtPaymentStatusDto;
use specta::Type;

/// Итог распределения дохода.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PlanBalanceDto {
    Empty,
    Balanced,
    Unallocated,
    Over,
}

impl From<PlanBalance> for PlanBalanceDto {
    fn from(b: PlanBalance) -> Self {
        match b {
            PlanBalance::Empty => Self::Empty,
            PlanBalance::Balanced => Self::Balanced,
            PlanBalance::Unallocated => Self::Unallocated,
            PlanBalance::Over => Self::Over,
        }
    }
}

/// Строка «план → факт» по статье или накоплению.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanRowDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub plan: i64,
    #[specta(type = specta_typescript::Number)]
    pub fact: i64,
    /// `fact − plan`: положительное — потрачено больше плана.
    #[specta(type = specta_typescript::Number)]
    pub deviation: i64,
}

impl From<&PlanRow> for PlanRowDto {
    fn from(r: &PlanRow) -> Self {
        Self {
            category_id: r.category_id.0,
            plan: r.plan.kopecks(),
            fact: r.fact.kopecks(),
            deviation: r.deviation.kopecks(),
        }
    }
}

/// Погашение долга в плане месяца: отдельная строка, отмечается «Оплачено» как трата.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanRepaymentDto {
    #[specta(type = specta_typescript::Number)]
    pub payment_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub debt_id: i64,
    pub lender: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
    pub status: DebtPaymentStatusDto,
}

/// План месяца: все числа посчитаны в Rust.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthPlanDto {
    pub month: String,
    /// Месяц зафиксирован кнопкой «План готов».
    pub locked: bool,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub plan_expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub plan_savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub plan_repayments: i64,
    /// `income − plan_expenses − plan_savings − plan_repayments`.
    #[specta(type = specta_typescript::Number)]
    pub unallocated: i64,
    pub balance: PlanBalanceDto,
    pub rows: Vec<PlanRowDto>,
    /// Погашения долгов этого месяца.
    pub repayments: Vec<PlanRepaymentDto>,
    /// Траты со статусом «Незапланировано».
    #[specta(type = specta_typescript::Number)]
    pub unplanned: i64,
    /// Взято в долг в этом месяце.
    #[specta(type = specta_typescript::Number)]
    pub borrowed: i64,
    /// Отложено (факт по накоплениям).
    #[specta(type = specta_typescript::Number)]
    pub saved: i64,
}

impl From<&MonthPlan> for MonthPlanDto {
    fn from(p: &MonthPlan) -> Self {
        Self {
            month: p.month.to_string(),
            locked: p.locked,
            income: p.income.kopecks(),
            plan_expenses: p.plan_expenses.kopecks(),
            plan_savings: p.plan_savings.kopecks(),
            plan_repayments: p.plan_repayments.kopecks(),
            unallocated: p.unallocated.kopecks(),
            balance: p.balance().into(),
            rows: p.rows.iter().map(PlanRowDto::from).collect(),
            repayments: Vec::new(),
            unplanned: p.unplanned.kopecks(),
            borrowed: p.borrowed.kopecks(),
            saved: p.saved.kopecks(),
        }
    }
}

/// Ожидаемое поступление в мастере первого месяца.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WizardIncomeDto {
    pub source_name: String,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
}

/// Плановая сумма по статье расходов.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WizardLineDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub amount: i64,
}

/// План накопления: процент от дохода или фиксированная сумма.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WizardSavingsPlanDto {
    Percent {
        rate_bp: i32,
    },
    Fixed {
        #[specta(type = specta_typescript::Number)]
        amount: i64,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WizardSavingsDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub plan: WizardSavingsPlanDto,
}

/// Ввод мастера первого месяца: доходы, суммы по статьям, план накоплений.
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanWizardInputDto {
    pub incomes: Vec<WizardIncomeDto>,
    pub lines: Vec<WizardLineDto>,
    pub savings: Vec<WizardSavingsDto>,
}

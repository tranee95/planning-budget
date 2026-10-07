//! Сводки месяца и года, лимиты, баланс бюджета (`core::calc`).

use planning_budget_core::Money;
use planning_budget_core::calc::{
    BudgetBalance, CategoryYearRow, Corridor, KindAmounts, LimitLevel, LimitRow, MonthSummary,
    StatusAmounts,
};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusAmountsDto {
    #[specta(type = specta_typescript::Number)]
    pub paid: i64,
    #[specta(type = specta_typescript::Number)]
    pub debt: i64,
    #[specta(type = specta_typescript::Number)]
    pub unplanned: i64,
    #[specta(type = specta_typescript::Number)]
    pub planned: i64,
    /// Сумма по всем статусам.
    #[specta(type = specta_typescript::Number)]
    pub total: i64,
    /// Доли статусов в сумме, базисные пункты (0 при нулевой сумме).
    pub share_bp: StatusSharesDto,
}

/// Доли статусов в общей сумме, базисные пункты (10 000 = 100 %).
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusSharesDto {
    pub paid: i32,
    pub debt: i32,
    pub unplanned: i32,
    pub planned: i32,
}

impl From<StatusAmounts> for StatusAmountsDto {
    fn from(a: StatusAmounts) -> Self {
        let shares = a.shares_bp();
        Self {
            paid: a.paid.kopecks(),
            debt: a.debt.kopecks(),
            unplanned: a.unplanned.kopecks(),
            planned: a.planned.kopecks(),
            total: a.total().map_or(i64::MAX, Money::kopecks),
            share_bp: StatusSharesDto {
                paid: shares.paid,
                debt: shares.debt,
                unplanned: shares.unplanned,
                planned: shares.planned,
            },
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KindAmountsDto {
    #[specta(type = specta_typescript::Number)]
    pub mandatory: i64,
    #[specta(type = specta_typescript::Number)]
    pub wants: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub loans: i64,
}

impl From<KindAmounts> for KindAmountsDto {
    fn from(a: KindAmounts) -> Self {
        Self {
            mandatory: a.mandatory.kopecks(),
            wants: a.wants.kopecks(),
            savings: a.savings.kopecks(),
            loans: a.loans.kopecks(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CorridorDto {
    NoIncome,
    Below,
    Within,
    Above,
}

impl From<Corridor> for CorridorDto {
    fn from(c: Corridor) -> Self {
        match c {
            Corridor::NoIncome => Self::NoIncome,
            Corridor::Below => Self::Below,
            Corridor::Within => Self::Within,
            Corridor::Above => Self::Above,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum LimitLevelDto {
    Ok,
    Warn,
    Over,
}

impl From<LimitLevel> for LimitLevelDto {
    fn from(l: LimitLevel) -> Self {
        match l {
            LimitLevel::Ok => Self::Ok,
            LimitLevel::Warn => Self::Warn,
            LimitLevel::Over => Self::Over,
        }
    }
}

/// Строка листа «Сводка» за месяц.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthSummaryDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub income_received: i64,
    #[specta(type = specta_typescript::Number)]
    pub income_expected: i64,
    #[specta(type = specta_typescript::Number)]
    pub expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    /// Долги, взятые в месяце: источник денег.
    #[specta(type = specta_typescript::Number)]
    pub borrowed: i64,
    /// Оплаченные погашения месяца: выплата, не расход.
    #[specta(type = specta_typescript::Number)]
    pub repaid: i64,
    #[specta(type = specta_typescript::Number)]
    pub free: i64,
    #[specta(type = specta_typescript::Number)]
    pub free_cum: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings_cum: i64,
    /// Норма сбережений в базисных пунктах; `None` без дохода.
    pub savings_rate_bp: Option<i32>,
    pub unspent_rate: Option<f64>,
    pub savings_plan_rate_bp: i32,
    /// Процент плана больше нуля и отличается от нормы из настроек.
    pub savings_plan_off_norm: bool,
    #[specta(type = specta_typescript::Number)]
    pub savings_plan: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings_gap: i64,
    #[specta(type = specta_typescript::Number)]
    pub per_week: i64,
    pub corridor: CorridorDto,
    #[specta(type = specta_typescript::Number)]
    pub top_up_to_min: i64,
    #[specta(type = specta_typescript::Number)]
    pub top_up_to_norm: i64,
    pub by_status: StatusAmountsDto,
    pub by_kind: KindAmountsDto,
}

impl From<&MonthSummary> for MonthSummaryDto {
    fn from(s: &MonthSummary) -> Self {
        Self {
            month: s.month.to_string(),
            income: s.income.kopecks(),
            income_received: s.income_received.kopecks(),
            income_expected: s.income_expected.kopecks(),
            expenses: s.expenses.kopecks(),
            savings: s.savings.kopecks(),
            borrowed: s.borrowed.kopecks(),
            repaid: s.repaid.kopecks(),
            free: s.free.kopecks(),
            free_cum: s.free_cum.kopecks(),
            savings_cum: s.savings_cum.kopecks(),
            savings_rate_bp: s.savings_rate_bp,
            unspent_rate: s.unspent_rate,
            savings_plan_rate_bp: s.savings_plan_rate.0,
            savings_plan_off_norm: s.savings_plan_off_norm,
            savings_plan: s.savings_plan.kopecks(),
            savings_gap: s.savings_gap.kopecks(),
            per_week: s.per_week.kopecks(),
            corridor: s.corridor.into(),
            top_up_to_min: s.top_up_to_min.kopecks(),
            top_up_to_norm: s.top_up_to_norm.kopecks(),
            by_status: s.by_status.into(),
            by_kind: s.by_kind.into(),
        }
    }
}

/// Категория в месяце: факт против лимита.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LimitRowDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub fact: i64,
    #[specta(type = Option<specta_typescript::Number>)]
    pub limit: Option<i64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub remaining: Option<i64>,
    pub usage: Option<f64>,
    /// `usage` в целых процентах.
    pub usage_percent: Option<i32>,
    /// Только у накоплений: процент плана по истории на месяц (без ручного значения месяца).
    pub plan_rate_bp: Option<i32>,
    /// Доля лимита, занятая оплаченным; `None` без лимита.
    pub paid_usage: Option<f64>,
    /// Доля лимита, занятая планом.
    pub planned_usage: Option<f64>,
    pub level: Option<LimitLevelDto>,
    pub by_status: StatusAmountsDto,
}

impl From<&LimitRow> for LimitRowDto {
    fn from(r: &LimitRow) -> Self {
        Self {
            category_id: r.category_id.0,
            fact: r.fact.kopecks(),
            limit: r.limit.map(Money::kopecks),
            remaining: r.remaining.map(Money::kopecks),
            usage: r.usage,
            usage_percent: r.usage_percent,
            plan_rate_bp: r.plan_rate_bp,
            paid_usage: r.paid_usage,
            planned_usage: r.planned_usage,
            level: r.level.map(Into::into),
            by_status: r.by_status.into(),
        }
    }
}

/// Период ряда месяцев для графика «Обзора».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
pub enum SeriesRangeDto {
    /// Двенадцать месяцев года выбранного месяца.
    #[serde(rename = "year")]
    Year,
    /// Двенадцать месяцев, заканчивая выбранным.
    #[serde(rename = "12m")]
    Last12,
    /// Все месяцы с данными (не дальше десяти лет назад).
    #[serde(rename = "all")]
    All,
}

/// Данные экрана «Обзор» за месяц.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MonthOverviewDto {
    pub summary: MonthSummaryDto,
    pub limits: Vec<LimitRowDto>,
    #[specta(type = specta_typescript::Number)]
    pub limits_total: i64,
    #[specta(type = specta_typescript::Number)]
    pub limits_remaining: i64,
    pub spent_vs_limits: Option<f64>,
    /// Категорий выше лимита.
    pub over_count: u32,
    /// План месяца зафиксирован («План готов»): новые траты по умолчанию «Незапланировано».
    pub plan_locked: bool,
    /// Расходы месяца к среднему за год в целых процентах; `None`, если сравнивать не с чем.
    pub expenses_delta_percent: Option<i32>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryYearRowDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    #[specta(type = specta_typescript::Number)]
    pub total: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg: i64,
    #[specta(type = Option<specta_typescript::Number>)]
    pub limit_now: Option<i64>,
    #[specta(type = Option<specta_typescript::Number>)]
    pub avg_minus_limit: Option<i64>,
    pub share_in_expenses: Option<f64>,
}

impl From<&CategoryYearRow> for CategoryYearRowDto {
    fn from(r: &CategoryYearRow) -> Self {
        Self {
            category_id: r.category_id.0,
            total: r.total.kopecks(),
            avg: r.avg.kopecks(),
            limit_now: r.limit_now.map(Money::kopecks),
            avg_minus_limit: r.avg_minus_limit.map(Money::kopecks),
            share_in_expenses: r.share_in_expenses,
        }
    }
}

/// «Баланс бюджета».
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetBalanceDto {
    #[specta(type = specta_typescript::Number)]
    pub avg_income: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings_target: i64,
    #[specta(type = specta_typescript::Number)]
    pub limits_sum: i64,
    #[specta(type = specta_typescript::Number)]
    pub buffer: i64,
    pub buffer_rate: Option<f64>,
    #[specta(type = specta_typescript::Number)]
    pub economy: i64,
}

impl From<&BudgetBalance> for BudgetBalanceDto {
    fn from(b: &BudgetBalance) -> Self {
        Self {
            avg_income: b.avg_income.kopecks(),
            savings_target: b.savings_target.kopecks(),
            limits_sum: b.limits_sum.kopecks(),
            buffer: b.buffer.kopecks(),
            buffer_rate: b.buffer_rate,
            economy: b.economy.kopecks(),
        }
    }
}

/// Все строки листа «Сводка» за год: месяцы, итоги, категории и баланс бюджета.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct YearSummaryDto {
    pub year: u16,
    pub months: Vec<MonthSummaryDto>,
    #[specta(type = specta_typescript::Number)]
    pub income: i64,
    #[specta(type = specta_typescript::Number)]
    pub expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub free: i64,
    pub months_with_data: u32,
    #[specta(type = specta_typescript::Number)]
    pub avg_income: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg_expenses: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg_savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub avg_free: i64,
    pub savings_rate: Option<f64>,
    pub by_status: StatusAmountsDto,
    pub by_kind: KindAmountsDto,
    pub categories: Vec<CategoryYearRowDto>,
    pub balance: BudgetBalanceDto,
}

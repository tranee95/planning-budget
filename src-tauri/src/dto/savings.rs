//! Раздел «Сбережения»: накопления, факт по месяцам, прогноз A/B/C.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::ChartDataDto;

/// Месяц фактического накопления.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsMonthDto {
    pub month: String,
    #[specta(type = specta_typescript::Number)]
    pub savings: i64,
    #[specta(type = specta_typescript::Number)]
    pub coupon: i64,
    #[specta(type = specta_typescript::Number)]
    pub balance: i64,
    #[specta(type = specta_typescript::Number)]
    pub deposited: i64,
    #[specta(type = specta_typescript::Number)]
    pub coupon_income: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ForecastKindDto {
    A,
    B,
    C,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ForecastScenarioDto {
    pub scenario: ForecastKindDto,
    /// Название сценария для таблиц и легенды: «A · цель нормы».
    pub name: String,
    #[specta(type = specta_typescript::Number)]
    pub contribution: i64,
    #[specta(type = specta_typescript::Number)]
    pub y1: i64,
    #[specta(type = specta_typescript::Number)]
    pub y3: i64,
    #[specta(type = specta_typescript::Number)]
    pub y5: i64,
    #[specta(type = specta_typescript::Number)]
    pub coupons60: i64,
    /// Баланс на конец каждого из 60 месяцев: точки линии прогноза.
    #[specta(type = Vec<specta_typescript::Number>)]
    pub balances: Vec<i64>,
}

/// Параметры накопления: ставка 0 — простое накопление без купонов.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsParamsDto {
    pub annual_rate_bp: i32,
    pub tax_bp: i32,
    #[specta(type = specta_typescript::Number)]
    pub initial_balance: i64,
    pub initial_month: String,
}

/// Как задан план накопления в месяц.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum SavingsPlanKindDto {
    Percent,
    Fixed,
}

/// Одно накопление: параметры, план, факт и прогноз; графики готовы к показу.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccumulationDto {
    #[specta(type = specta_typescript::Number)]
    pub category_id: i64,
    pub params: SavingsParamsDto,
    pub plan_kind: SavingsPlanKindDto,
    /// Процент плана от дохода, базисные пункты (при `planKind = percent`).
    pub plan_rate_bp: i32,
    /// Фиксированная сумма плана в месяц (при `planKind = fixed`).
    #[specta(type = Option<specta_typescript::Number>)]
    pub plan_fixed: Option<i64>,
    /// План взноса в текущем месяце.
    #[specta(type = specta_typescript::Number)]
    pub plan: i64,
    /// Баланс на конец декабря года.
    #[specta(type = specta_typescript::Number)]
    pub balance: i64,
    /// Ставка после налога на купон, базисные пункты: 1600 при налоге 1300 дают 1392.
    pub effective_rate_bp: i32,
    pub months: Vec<SavingsMonthDto>,
    pub scenarios: Vec<ForecastScenarioDto>,
    /// Линии «Баланс» и «Внесено» по месяцам года.
    pub fact_chart: ChartDataDto,
    /// Линии сценариев A/B/C на 60 месяцев.
    pub forecast_chart: ChartDataDto,
}

/// Раздел «Сбережения» за год.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavingsOverviewDto {
    pub year: u16,
    /// Самый ранний год, с которого у накоплений есть данные (нижняя граница переключателя года).
    pub first_year: u16,
    pub items: Vec<AccumulationDto>,
    /// Сумма балансов накоплений на конец декабря.
    #[specta(type = specta_typescript::Number)]
    pub total_balance: i64,
    /// План сбережений на декабрь года просмотра (сумма планов накоплений).
    #[specta(type = specta_typescript::Number)]
    pub month_plan: i64,
    pub total_scenarios: Vec<ForecastScenarioDto>,
    pub total_forecast_chart: ChartDataDto,
}

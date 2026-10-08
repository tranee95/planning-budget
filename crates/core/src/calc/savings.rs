//! Накопления: баланс и прогноз по каждой категории-сбережению.

use super::bonds::forecast;
use super::{BondRates, BondsFact, BondsForecast, ForecastScenario, Ledger, Scenario};
use crate::error::CoreError;
use crate::model::{BasisPoints, CategoryId, CategoryKind, SavingsParams};
use crate::money::Money;
use crate::period::YearMonth;

/// Одно накопление: категория-сбережение с параметрами, фактом и прогнозом.
#[derive(Clone, PartialEq, Debug)]
pub struct Accumulation {
    pub category_id: CategoryId,
    pub params: SavingsParams,
    /// План взноса в месяце расчёта (декабрь года просмотра): процент от среднего дохода года
    /// или фиксированная сумма.
    pub plan: Money,
    /// Годовая ставка после налога на купон, `rate × (1 − tax)`, в базисных пунктах.
    pub effective_rate: BasisPoints,
    /// Действующая на `current` строка плана: процент от дохода (при `plan_fixed = None`) …
    pub plan_rate: BasisPoints,
    /// … либо фиксированная сумма.
    pub plan_fixed: Option<Money>,
    pub fact: BondsFact,
    pub forecast: BondsForecast,
}

/// Величины года, общие для прогнозов всех накоплений.
struct ForecastContext {
    months_with_data: u32,
    avg_income: Money,
    economy: Money,
    /// Сумма планов неархивных накоплений: по ней делится экономия от лимитов (сценарий B).
    plan_sum: Money,
}

/// Раздел «Сбережения»: накопления и итоги по ним.
#[derive(Clone, PartialEq, Debug)]
pub struct SavingsOverview {
    pub items: Vec<Accumulation>,
    /// Сумма балансов накоплений на конец декабря года.
    pub total_balance: Money,
    /// План сбережений месяца расчёта (декабрь года просмотра) по всем категориям-сбережениям.
    pub month_plan: Money,
    /// Прогноз по всем накоплениям: суммы вкладов, итогов и точек линии.
    pub total_forecast: [ForecastScenario; 3],
}

impl Ledger<'_> {
    /// Параметры накопления; у категории без строки параметров — простое накопление с нуля с января `year`.
    fn params_or_default(
        &self,
        category: CategoryId,
        year: u16,
    ) -> Result<SavingsParams, CoreError> {
        match self.data().savings_params.get(&category) {
            Some(p) => Ok(*p),
            None => Ok(SavingsParams {
                annual_rate: BasisPoints(0),
                tax: BasisPoints(0),
                initial_balance: Money::ZERO,
                initial_month: YearMonth::first_of_year(year)?,
            }),
        }
    }

    /// Баланс накопления по месяцам `year`: взносы — траты категории (все статусы), накопление идёт от
    /// `initial_month` его параметров.
    pub fn accumulation_fact(
        &self,
        category: CategoryId,
        year: u16,
    ) -> Result<BondsFact, CoreError> {
        let params = self.params_or_default(category, year)?;
        self.accumulate(
            BondRates::new(params.annual_rate, params.tax),
            params.initial_balance,
            params.initial_month,
            year,
            |month| Ok(self.category_total(month, category)?),
        )
    }

    /// Плановый взнос накопления на «типичный» месяц: процент от среднего дохода года либо
    /// фиксированная сумма.
    fn typical_plan(
        &self,
        category: CategoryId,
        current: YearMonth,
        avg_income: Money,
    ) -> Result<Money, CoreError> {
        let Some(entry) = self.plan_entry(current, category) else {
            return Ok(Money::ZERO);
        };
        match entry.fixed {
            Some(amount) => Ok(amount),
            None => Ok(avg_income.mul_ratio(entry.rate.as_ratio())?),
        }
    }

    /// Общее для прогнозов всех накоплений года: считается один раз.
    fn forecast_context(
        &self,
        year: u16,
        current: YearMonth,
    ) -> Result<ForecastContext, CoreError> {
        let summary = self.year_summary(year, current)?;
        let balance = self.balance_for(&summary, current)?;
        let mut plan_sum = Money::ZERO;
        for category in self
            .data()
            .categories
            .iter()
            .filter(|c| c.kind == CategoryKind::Savings && !c.archived)
        {
            plan_sum = plan_sum.checked_add(self.typical_plan(
                category.id,
                current,
                balance.avg_income,
            )?)?;
        }
        Ok(ForecastContext {
            months_with_data: summary.months_with_data,
            avg_income: balance.avg_income,
            economy: balance.economy,
            plan_sum,
        })
    }

    /// Прогноз накопления на 60 месяцев.
    pub fn accumulation_forecast(
        &self,
        category: CategoryId,
        year: u16,
        current: YearMonth,
    ) -> Result<BondsForecast, CoreError> {
        let start_balance = self.accumulation_fact(category, year)?.dec_balance;
        let context = self.forecast_context(year, current)?;
        self.forecast_with(&context, category, year, current, start_balance)
    }

    fn forecast_with(
        &self,
        context: &ForecastContext,
        category: CategoryId,
        year: u16,
        current: YearMonth,
        start_balance: Money,
    ) -> Result<BondsForecast, CoreError> {
        let params = self.params_or_default(category, year)?;
        let rates = BondRates::new(params.annual_rate, params.tax);
        let plan = self.typical_plan(category, current, context.avg_income)?;
        let economy_share = match plan.ratio(context.plan_sum) {
            Some(share) => context.economy.mul_ratio(share)?,
            None => Money::ZERO,
        };
        let divisor = i64::from(context.months_with_data);
        let actual = self
            .category_year_total(year, current, category)?
            .div_round(divisor)?;

        let [a, b, c] = [
            (Scenario::A, plan),
            (Scenario::B, plan.checked_add(economy_share)?),
            (Scenario::C, actual),
        ]
        .map(|(scenario, contribution)| forecast(scenario, contribution, start_balance, rates));
        Ok(BondsForecast {
            start_balance,
            scenarios: [a?, b?, c?],
        })
    }

    /// Накопления года `year`: у каждого факт и прогноз, плюс общие итоги. Архивные категории входят,
    /// только если на них остался баланс. Планы и средние считаются на декабрь `year`: одинаково для
    /// прошлого, текущего и будущего года, независимо от сегодняшней даты.
    pub fn savings_overview(&self, year: u16) -> Result<SavingsOverview, CoreError> {
        let current = YearMonth::new(year, 12)?;
        let context = self.forecast_context(year, current)?;
        let mut items = Vec::new();
        for category in self
            .data()
            .categories
            .iter()
            .filter(|c| c.kind == CategoryKind::Savings)
        {
            let fact = self.accumulation_fact(category.id, year)?;
            if category.archived && fact.dec_balance.is_zero() {
                continue;
            }
            let params = self.params_or_default(category.id, year)?;
            let entry = self.plan_entry(current, category.id);
            items.push(Accumulation {
                category_id: category.id,
                params,
                plan_rate: entry.map_or(BasisPoints(0), |e| e.rate),
                plan_fixed: entry.and_then(|e| e.fixed),
                plan: self.typical_plan(category.id, current, context.avg_income)?,
                effective_rate: BondRates::new(params.annual_rate, params.tax).effective_bp(),
                forecast: self.forecast_with(
                    &context,
                    category.id,
                    year,
                    current,
                    fact.dec_balance,
                )?,
                fact,
            });
        }
        let mut total_balance = Money::ZERO;
        for item in &items {
            total_balance = total_balance.checked_add(item.fact.dec_balance)?;
        }
        let total_forecast = [Scenario::A, Scenario::B, Scenario::C].map(|scenario| {
            let mut sum = ForecastScenario {
                scenario,
                contribution: Money::ZERO,
                y1: Money::ZERO,
                y3: Money::ZERO,
                y5: Money::ZERO,
                coupons_60: Money::ZERO,
                balances: Vec::new(),
            };
            for item in &items {
                let Some(part) = item
                    .forecast
                    .scenarios
                    .iter()
                    .find(|s| s.scenario == scenario)
                else {
                    continue;
                };
                sum.contribution = sum.contribution.checked_add(part.contribution)?;
                sum.y1 = sum.y1.checked_add(part.y1)?;
                sum.y3 = sum.y3.checked_add(part.y3)?;
                sum.y5 = sum.y5.checked_add(part.y5)?;
                sum.coupons_60 = sum.coupons_60.checked_add(part.coupons_60)?;
                if sum.balances.is_empty() {
                    sum.balances = part.balances.clone();
                } else {
                    for (total, value) in sum.balances.iter_mut().zip(&part.balances) {
                        *total = total.checked_add(*value)?;
                    }
                }
            }
            Ok::<_, CoreError>(sum)
        });
        let [a, b, c] = total_forecast;
        Ok(SavingsOverview {
            items,
            total_balance,
            month_plan: self.savings_plan(current)?,
            total_forecast: [a?, b?, c?],
        })
    }
}

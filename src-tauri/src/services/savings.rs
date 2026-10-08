//! Раздел «Сбережения» поверх `core::calc` и хранилища.

use planning_budget_core::calc::{Accumulation, ForecastScenario, Ledger, Scenario};
use planning_budget_core::{BasisPoints, CategoryId, Money, SavingsParams, YearMonth};
use planning_budget_storage::Db;

use super::parse_month;
use crate::AppError;
use crate::dto::{
    AccumulationDto, ChartDataDto, ChartSeriesDto, ChartUnitDto, ForecastKindDto,
    ForecastScenarioDto, SavingsMonthDto, SavingsOverviewDto, SavingsParamsDto, SavingsPlanKindDto,
};

fn scenario_kind(s: Scenario) -> ForecastKindDto {
    match s {
        Scenario::A => ForecastKindDto::A,
        Scenario::B => ForecastKindDto::B,
        Scenario::C => ForecastKindDto::C,
    }
}

fn scenario_name(s: Scenario) -> &'static str {
    match s {
        Scenario::A => "A · цель нормы",
        Scenario::B => "B · норма и экономия",
        Scenario::C => "C · как сейчас",
    }
}

fn scenario_token(s: Scenario) -> &'static str {
    match s {
        Scenario::A => "scenario.a.forecast",
        Scenario::B => "scenario.b.forecast",
        Scenario::C => "scenario.c.forecast",
    }
}

fn scenario_dto(s: &ForecastScenario) -> ForecastScenarioDto {
    ForecastScenarioDto {
        scenario: scenario_kind(s.scenario),
        name: scenario_name(s.scenario).to_owned(),
        contribution: s.contribution.kopecks(),
        y1: s.y1.kopecks(),
        y3: s.y3.kopecks(),
        y5: s.y5.kopecks(),
        coupons60: s.coupons_60.kopecks(),
        balances: s.balances.iter().map(|m| m.kopecks()).collect(),
    }
}

/// Рубли для оси графика (значения `ChartData` — в рублях).
fn rub(m: Money) -> Option<f64> {
    Some(m.as_f64() / 100.0)
}

/// «Янв» — первые три буквы названия месяца.
fn short_month(month: YearMonth) -> String {
    month.month_name_ru().chars().take(3).collect()
}

fn fact_chart(months: &[SavingsMonthDto], parsed: &[YearMonth]) -> ChartDataDto {
    let values = |f: fn(&SavingsMonthDto) -> i64| {
        months
            .iter()
            .map(|m| Some(Money::from_kopecks(f(m)).as_f64() / 100.0))
            .collect()
    };
    ChartDataDto {
        categories: parsed.iter().map(|m| short_month(*m)).collect(),
        category_tokens: months
            .iter()
            .map(|m| format!("month:{}", m.month))
            .collect(),
        series: vec![
            ChartSeriesDto {
                name: "Баланс".to_owned(),
                color: "bonds.balance".to_owned(),
                values: values(|m| m.balance),
            },
            ChartSeriesDto {
                name: "Внесено".to_owned(),
                color: "bonds.deposited".to_owned(),
                values: values(|m| m.deposited),
            },
        ],
        reference_lines: Vec::new(),
        totals: None,
        unit: ChartUnitDto::Rub,
    }
}

/// Прогноз начинается с января года, следующего за `year`; подписи — «Янв 2027».
fn forecast_chart(year: u16, scenarios: &[ForecastScenario]) -> Result<ChartDataDto, AppError> {
    let first = YearMonth::new(year.saturating_add(1), 1)?;
    let length = scenarios.first().map_or(0, |s| s.balances.len());
    let months: Vec<YearMonth> = std::iter::successors(Some(first), |m| m.succ())
        .take(length)
        .collect();
    Ok(ChartDataDto {
        categories: months
            .iter()
            .map(|m| format!("{} {}", short_month(*m), m.year()))
            .collect(),
        category_tokens: months.iter().map(|_| String::new()).collect(),
        series: scenarios
            .iter()
            .map(|s| ChartSeriesDto {
                name: scenario_name(s.scenario).to_owned(),
                color: scenario_token(s.scenario).to_owned(),
                values: s.balances.iter().map(|b| rub(*b)).collect(),
            })
            .collect(),
        reference_lines: Vec::new(),
        totals: None,
        unit: ChartUnitDto::Rub,
    })
}

fn params_dto(p: &SavingsParams) -> SavingsParamsDto {
    SavingsParamsDto {
        annual_rate_bp: p.annual_rate.0,
        tax_bp: p.tax.0,
        initial_balance: p.initial_balance.kopecks(),
        initial_month: p.initial_month.to_string(),
    }
}

fn accumulation_dto(a: &Accumulation, year: u16) -> Result<AccumulationDto, AppError> {
    let months: Vec<SavingsMonthDto> = a
        .fact
        .months
        .iter()
        .map(|m| SavingsMonthDto {
            month: m.month.to_string(),
            savings: m.savings.kopecks(),
            coupon: m.coupon.kopecks(),
            balance: m.balance.kopecks(),
            deposited: m.deposited.kopecks(),
            coupon_income: m.coupon_income.kopecks(),
        })
        .collect();
    let parsed: Vec<YearMonth> = a.fact.months.iter().map(|m| m.month).collect();
    Ok(AccumulationDto {
        category_id: a.category_id.0,
        params: params_dto(&a.params),
        plan_kind: if a.plan_fixed.is_some() {
            SavingsPlanKindDto::Fixed
        } else {
            SavingsPlanKindDto::Percent
        },
        plan_rate_bp: a.plan_rate.0,
        plan_fixed: a.plan_fixed.map(Money::kopecks),
        plan: a.plan.kopecks(),
        balance: a.fact.dec_balance.kopecks(),
        effective_rate_bp: a.effective_rate.0,
        fact_chart: fact_chart(&months, &parsed),
        months,
        forecast_chart: forecast_chart(year, &a.forecast.scenarios)?,
        scenarios: a.forecast.scenarios.iter().map(scenario_dto).collect(),
    })
}

/// Накопления года `year`: у каждого факт и прогноз, плюс общие итоги и готовые графики.
/// Накопление идёт от своего стартового месяца, поэтому данные читаются с самого раннего из них
/// (или с января года, если он раньше) по декабрь. Планы считаются на декабрь года просмотра.
pub fn overview(db: &Db, year: u16) -> Result<SavingsOverviewDto, AppError> {
    let first = YearMonth::first_of_year(year)?;
    let earliest = db
        .savings_params()?
        .values()
        .map(|p| p.initial_month)
        .min()
        .unwrap_or(first);
    let data = db.dataset(earliest.min(first), YearMonth::new(year, 12)?)?;
    let ledger = Ledger::new(&data)?;
    let report = ledger.savings_overview(year)?;
    let items = report
        .items
        .iter()
        .map(|a| accumulation_dto(a, year))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SavingsOverviewDto {
        year,
        first_year: earliest.year().min(year),
        items,
        total_balance: report.total_balance.kopecks(),
        month_plan: report.month_plan.kopecks(),
        total_scenarios: report.total_forecast.iter().map(scenario_dto).collect(),
        total_forecast_chart: forecast_chart(year, &report.total_forecast)?,
    })
}

pub fn params_set(
    db: &mut Db,
    category_id: i64,
    params: &SavingsParamsDto,
) -> Result<(), AppError> {
    db.savings_params_set(
        CategoryId(category_id),
        &SavingsParams {
            annual_rate: BasisPoints(params.annual_rate_bp),
            tax: BasisPoints(params.tax_bp),
            initial_balance: Money::from_kopecks(params.initial_balance),
            initial_month: parse_month(&params.initial_month, "initialMonth")?,
        },
    )?;
    Ok(())
}

pub fn fixed_set(
    db: &mut Db,
    category_id: i64,
    valid_from: &str,
    amount: i64,
) -> Result<(), AppError> {
    db.savings_fixed_set(
        CategoryId(category_id),
        parse_month(valid_from, "validFrom")?,
        Money::from_kopecks(amount),
    )?;
    Ok(())
}

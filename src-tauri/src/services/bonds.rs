//! Облигации: факт по месяцам и прогноз A/B/C поверх `core::calc`.

use budget_core::YearMonth;
use budget_core::calc::{BondRates, ForecastScenario, Ledger, Scenario};
use budget_storage::Db;
use chrono::NaiveDate;

use super::month_of;
use crate::AppError;
use crate::dto::{BondsDto, BondsMonthDto, ForecastKindDto, ForecastScenarioDto};

fn scenario_dto(s: &ForecastScenario) -> ForecastScenarioDto {
    ForecastScenarioDto {
        scenario: match s.scenario {
            Scenario::A => ForecastKindDto::A,
            Scenario::B => ForecastKindDto::B,
            Scenario::C => ForecastKindDto::C,
        },
        contribution: s.contribution.kopecks(),
        y1: s.y1.kopecks(),
        y3: s.y3.kopecks(),
        y5: s.y5.kopecks(),
        coupons60: s.coupons_60.kopecks(),
        balances: s.balances.iter().map(|m| m.kopecks()).collect(),
    }
}

/// Факт за год `year` и прогноз от декабрьского баланса. Накопление идёт от стартового месяца
/// настроек, поэтому читаются данные с него (или с января года, если он раньше) по декабрь.
pub fn projection(db: &Db, year: u16, today: NaiveDate) -> Result<BondsDto, AppError> {
    let current = month_of(today)?;
    let first = YearMonth::first_of_year(year)?;
    let start = db.settings()?.bonds_initial_month.min(first);
    let data = db.dataset(start, YearMonth::new(year, 12)?)?;
    let ledger = Ledger::new(&data)?;
    let fact = ledger.bonds_fact(year)?;
    let forecast = ledger.bonds_forecast(year, current)?;
    let rates = BondRates::from_settings(&data.settings);
    Ok(BondsDto {
        year,
        effective_rate: rates.effective,
        monthly_rate: rates.monthly,
        months: fact
            .months
            .iter()
            .map(|m| BondsMonthDto {
                month: m.month.to_string(),
                savings: m.savings.kopecks(),
                coupon: m.coupon.kopecks(),
                balance: m.balance.kopecks(),
                deposited: m.deposited.kopecks(),
                coupon_income: m.coupon_income.kopecks(),
            })
            .collect(),
        dec_balance: forecast.start_balance.kopecks(),
        scenarios: forecast.scenarios.iter().map(scenario_dto).collect(),
    })
}

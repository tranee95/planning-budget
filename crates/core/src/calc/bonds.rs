//! Облигации: факт за год и прогноз на 60 месяцев.
//!
//! Баланс и купоны считаются в `f64` без промежуточного округления (как в таблице);
//! в `Money` значения округляются один раз при выводе.

use super::Ledger;
use crate::error::CoreError;
use crate::model::BasisPoints;
use crate::money::Money;
use crate::period::YearMonth;

const FORECAST_MONTHS: u32 = 60;

/// Эффективная и месячная ставки после налога на купон.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BondRates {
    /// `rate × (1 − tax)`.
    pub effective: f64,
    /// `(1 + effective)^(1/12) − 1`.
    pub monthly: f64,
}

impl BondRates {
    /// Ставки накопления: годовая ставка и налог на купон.
    pub fn new(annual_rate: BasisPoints, tax: BasisPoints) -> Self {
        let effective = annual_rate.as_ratio() * (1.0 - tax.as_ratio());
        let monthly = (1.0 + effective).powf(1.0 / 12.0) - 1.0;
        Self { effective, monthly }
    }
}

/// Месяц фактического накопления.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BondsMonth {
    pub month: YearMonth,
    pub savings: Money,
    /// Купон месяца: `баланс прошлого месяца × monthly`.
    pub coupon: Money,
    pub balance: Money,
    /// Начальный баланс + взносы с начала учёта.
    pub deposited: Money,
    /// `balance − deposited`.
    pub coupon_income: Money,
}

/// Факт за год: месяцы от `initial_month` (или января) по декабрь.
#[derive(Clone, PartialEq, Debug)]
pub struct BondsFact {
    pub months: Vec<BondsMonth>,
    /// Баланс на конец декабря (при пустом списке месяцев — начальный).
    pub dec_balance: Money,
}

/// Сценарий прогноза.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scenario {
    /// Цель нормы.
    A,
    /// Норма плюс экономия от лимитов.
    B,
    /// Как сейчас: средние сбережения года.
    C,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ForecastScenario {
    pub scenario: Scenario,
    pub contribution: Money,
    pub y1: Money,
    pub y3: Money,
    pub y5: Money,
    /// `b_60 − b_0 − 60 × contribution`.
    pub coupons_60: Money,
    /// Баланс на конец каждого из 60 месяцев (`b_1 … b_60`): точки линии прогноза.
    pub balances: Vec<Money>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BondsForecast {
    pub start_balance: Money,
    pub scenarios: [ForecastScenario; 3],
}

impl Ledger<'_> {
    /// Баланс по взносам `contribution(m)` с `initial_month` по декабрь `year`.
    pub(crate) fn accumulate(
        &self,
        rates: BondRates,
        initial_balance: Money,
        initial_month: YearMonth,
        year: u16,
        contribution: impl Fn(YearMonth) -> Result<Money, CoreError>,
    ) -> Result<BondsFact, CoreError> {
        let end = YearMonth::new(year, 12)?;

        let mut balance = initial_balance.as_f64();
        let mut deposited = initial_balance;
        let mut months = Vec::new();
        for month in initial_month.iter_to(end) {
            let savings = contribution(month)?;
            let coupon = balance * rates.monthly;
            balance += savings.as_f64() + coupon;
            deposited = deposited.checked_add(savings)?;
            if month.year() != year {
                continue;
            }
            let balance_money = Money::from_kopecks_f64(balance)?;
            months.push(BondsMonth {
                month,
                savings,
                coupon: Money::from_kopecks_f64(coupon)?,
                balance: balance_money,
                deposited,
                coupon_income: balance_money.checked_sub(deposited)?,
            });
        }
        Ok(BondsFact {
            months,
            dec_balance: Money::from_kopecks_f64(balance)?,
        })
    }
}

pub(crate) fn forecast(
    scenario: Scenario,
    contribution: Money,
    start_balance: Money,
    rates: BondRates,
) -> Result<ForecastScenario, CoreError> {
    let mut balance = start_balance.as_f64();
    let (mut y1, mut y3) = (Money::ZERO, Money::ZERO);
    let mut balances = Vec::with_capacity(60);
    for n in 1..=FORECAST_MONTHS {
        balance = balance * (1.0 + rates.monthly) + contribution.as_f64();
        balances.push(Money::from_kopecks_f64(balance)?);
        match n {
            12 => y1 = Money::from_kopecks_f64(balance)?,
            36 => y3 = Money::from_kopecks_f64(balance)?,
            _ => {}
        }
    }
    let paid_in = contribution.as_f64() * f64::from(FORECAST_MONTHS);
    Ok(ForecastScenario {
        scenario,
        contribution,
        y1,
        y3,
        y5: Money::from_kopecks_f64(balance)?,
        coupons_60: Money::from_kopecks_f64(balance - start_balance.as_f64() - paid_in)?,
        balances,
    })
}

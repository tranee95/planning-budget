//! Облигации: факт за год и прогноз на 60 месяцев.
//!
//! Баланс и купоны считаются в `f64` без промежуточного округления (как в таблице);
//! в `Money` значения округляются один раз при выводе.

use super::Ledger;
use crate::error::CoreError;
use crate::model::Settings;
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
    pub fn from_settings(settings: &Settings) -> Self {
        let effective =
            settings.bonds_rate.as_ratio() * (1.0 - settings.bonds_coupon_tax.as_ratio());
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
    /// Фактический баланс по месяцам года `year`. Накопление всегда идёт от `initial_month`:
    /// взносы и купоны прошлых лет входят в баланс, в список попадают только месяцы `year`.
    pub fn bonds_fact(&self, year: u16) -> Result<BondsFact, CoreError> {
        let settings = &self.data().settings;
        let rates = BondRates::from_settings(settings);
        let end = YearMonth::new(year, 12)?;

        let mut balance = settings.bonds_initial_balance.as_f64();
        let mut deposited = settings.bonds_initial_balance;
        let mut months = Vec::new();
        for month in settings.bonds_initial_month.iter_to(end) {
            let savings = self.totals(month).by_kind.savings;
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

    /// Прогноз на 60 месяцев от декабрьского баланса `year`; `current` задаёт действующие лимиты.
    pub fn bonds_forecast(
        &self,
        year: u16,
        current: YearMonth,
    ) -> Result<BondsForecast, CoreError> {
        let rates = BondRates::from_settings(&self.data().settings);
        let start_balance = self.bonds_fact(year)?.dec_balance;
        let summary = self.year_summary(year, current)?;
        let balance = self.balance_for(&summary, current)?;

        let contributions = [
            (Scenario::A, balance.savings_target),
            (
                Scenario::B,
                balance.savings_target.checked_add(balance.economy)?,
            ),
            (Scenario::C, summary.avg_savings),
        ];
        let [a, b, c] = contributions
            .map(|(scenario, contribution)| forecast(scenario, contribution, start_balance, rates));
        Ok(BondsForecast {
            start_balance,
            scenarios: [a?, b?, c?],
        })
    }
}

fn forecast(
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

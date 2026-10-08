//! Накопления: план процентом или суммой, баланс и прогноз по каждому.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::calc::Ledger;
use planning_budget_core::{
    BasisPoints, Category, CategoryId, CategoryKind, DataSet, Income, IncomeId, IncomeStatus,
    Money, SavingsParams, SavingsRateEntry, Settings, Transaction, TxId, TxStatus, YearMonth,
};

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

const CUSHION: CategoryId = CategoryId(1);
const TRIP: CategoryId = CategoryId(2);
const BONDS: CategoryId = CategoryId(3);

fn base() -> DataSet {
    let category = |id, name: &str| Category {
        id,
        name: name.to_owned(),
        kind: CategoryKind::Savings,
        color: "#000000".to_owned(),
        sort_order: 0,
        note: None,
        archived: false,
    };
    let entry = |category_id, rate: i32, fixed: Option<Money>| SavingsRateEntry {
        category_id,
        valid_from: ym("2026-01"),
        rate: BasisPoints(rate),
        fixed_amount: fixed,
    };
    let params = |rate: i32, tax: i32| SavingsParams {
        annual_rate: BasisPoints(rate),
        tax: BasisPoints(tax),
        initial_balance: Money::ZERO,
        initial_month: ym("2026-01"),
    };
    DataSet {
        settings: Settings {
            savings_min: BasisPoints(1300),
            savings_norm: BasisPoints(1400),
            savings_max: BasisPoints(1500),
            weeks_per_month: 4,
        },
        categories: vec![
            category(CUSHION, "Подушка"),
            category(TRIP, "Отпуск"),
            category(BONDS, "Облигации"),
        ],
        limits: Vec::new(),
        // Подушка 4 % от дохода, «Отпуск» 5 000 ₽ фиксированно, «Облигации» 8 %.
        savings_rates: vec![
            entry(CUSHION, 400, None),
            entry(TRIP, 0, Some(rub(5_000))),
            entry(BONDS, 800, None),
        ],
        savings_overrides: BTreeMap::new(),
        transactions: Vec::new(),
        incomes: (1..=3)
            .map(|m| Income {
                id: IncomeId(m),
                month: ym(&format!("2026-{m:02}")),
                source_name: "Зарплата".to_owned(),
                amount: rub(100_000),
                status: IncomeStatus::Received,
            })
            .collect(),
        debts: Vec::new(),
        debt_payments: Vec::new(),
        locked_plans: BTreeMap::new(),
        savings_params: BTreeMap::from([
            (CUSHION, params(0, 0)),
            (TRIP, params(0, 0)),
            (BONDS, params(1600, 1300)),
        ]),
    }
}

fn deposit(data: &mut DataSet, month: &str, category: CategoryId, amount: i64) {
    let id = TxId(i64::try_from(data.transactions.len()).unwrap() + 1);
    data.transactions.push(Transaction {
        id,
        month: ym(month),
        category_id: category,
        title: "Взнос".to_owned(),
        amount: rub(amount),
        status: TxStatus::Paid,
        planned_amount: None,
    });
}

#[test]
fn plan_is_percent_of_income_or_a_fixed_amount_per_accumulation() {
    let data = base();
    let ledger = Ledger::new(&data).unwrap();
    let plan = |c| ledger.category_savings_plan(ym("2026-02"), c).unwrap();
    assert_eq!(plan(CUSHION), rub(4_000));
    assert_eq!(plan(TRIP), rub(5_000));
    assert_eq!(plan(BONDS), rub(8_000));
    assert_eq!(ledger.savings_plan(ym("2026-02")).unwrap(), rub(17_000));
}

#[test]
fn fixed_plan_does_not_depend_on_income_and_exists_in_an_empty_month() {
    let data = base();
    let ledger = Ledger::new(&data).unwrap();
    // В апреле дохода нет: процентные планы нулевые, фиксированная сумма остаётся.
    assert_eq!(
        ledger.category_savings_plan(ym("2026-04"), TRIP).unwrap(),
        rub(5_000)
    );
    assert_eq!(
        ledger
            .category_savings_plan(ym("2026-04"), CUSHION)
            .unwrap(),
        Money::ZERO
    );
}

#[test]
fn month_override_in_percent_wins_over_a_fixed_plan() {
    let mut data = base();
    data.savings_overrides
        .insert((ym("2026-02"), TRIP), BasisPoints(1000));
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(
        ledger.category_savings_plan(ym("2026-02"), TRIP).unwrap(),
        rub(10_000)
    );
    assert_eq!(
        ledger.category_savings_plan(ym("2026-03"), TRIP).unwrap(),
        rub(5_000)
    );
}

#[test]
fn fixed_plan_counts_zero_percent_in_the_total_rate() {
    let data = base();
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(ledger.savings_plan_rate(ym("2026-02")), BasisPoints(1200));
    // Для строки «Отпуск» процента по истории нет: в строке лимитов он не показывается.
    assert_eq!(ledger.category_rate_from_history(ym("2026-02"), TRIP), None);
    assert_eq!(
        ledger.category_rate_from_history(ym("2026-02"), CUSHION),
        Some(BasisPoints(400))
    );
}

#[test]
fn zero_rate_accumulation_is_a_plain_running_sum() {
    let mut data = base();
    deposit(&mut data, "2026-01", CUSHION, 4_000);
    deposit(&mut data, "2026-02", CUSHION, 4_000);
    deposit(&mut data, "2026-03", CUSHION, 1_000);
    let fact = Ledger::new(&data)
        .unwrap()
        .accumulation_fact(CUSHION, 2026)
        .unwrap();
    let balances: Vec<i64> = fact
        .months
        .iter()
        .map(|m| m.balance.kopecks() / 100)
        .collect();
    assert_eq!(&balances[..4], &[4_000, 8_000, 9_000, 9_000]);
    assert!(fact.months.iter().all(|m| m.coupon == Money::ZERO));
    assert_eq!(fact.dec_balance, rub(9_000));
    assert_eq!(fact.months[2].coupon_income, Money::ZERO);
}

#[test]
fn accumulations_are_independent_and_totals_add_up() {
    let mut data = base();
    deposit(&mut data, "2026-01", CUSHION, 4_000);
    deposit(&mut data, "2026-01", BONDS, 8_000);
    deposit(&mut data, "2026-02", BONDS, 8_000);
    let ledger = Ledger::new(&data).unwrap();
    let cushion = ledger.accumulation_fact(CUSHION, 2026).unwrap();
    let bonds = ledger.accumulation_fact(BONDS, 2026).unwrap();
    assert_eq!(cushion.dec_balance, rub(4_000));
    // «Облигации» растут на купонах: баланс выше внесённых 16 000 ₽.
    assert!(bonds.dec_balance > rub(16_000));

    let overview = ledger.savings_overview(2026).unwrap();
    assert_eq!(overview.items.len(), 3);
    let sum: i64 = overview
        .items
        .iter()
        .map(|i| i.fact.dec_balance.kopecks())
        .sum();
    assert_eq!(overview.total_balance.kopecks(), sum);
    for scenario in 0..3 {
        let parts: i64 = overview
            .items
            .iter()
            .map(|i| i.forecast.scenarios[scenario].y5.kopecks())
            .sum();
        assert_eq!(overview.total_forecast[scenario].y5.kopecks(), parts);
        assert_eq!(overview.total_forecast[scenario].balances.len(), 60);
    }
}

#[test]
fn archived_accumulation_without_balance_is_left_out_of_the_overview() {
    let mut data = base();
    data.categories[1].archived = true;
    let overview = Ledger::new(&data).unwrap().savings_overview(2026).unwrap();
    assert_eq!(overview.items.len(), 2);

    deposit(&mut data, "2026-01", TRIP, 5_000);
    let overview = Ledger::new(&data).unwrap().savings_overview(2026).unwrap();
    assert_eq!(
        overview.items.len(),
        3,
        "архивное накопление с балансом остаётся видимым"
    );
}

#[test]
fn year_before_initial_month_shows_zero_balance_and_forecast_from_zero() {
    let mut data = base();
    let params = data.savings_params.get_mut(&BONDS).unwrap();
    params.initial_balance = rub(100_000);
    params.initial_month = ym("2027-03");
    let overview = Ledger::new(&data).unwrap().savings_overview(2026).unwrap();
    let bonds = overview
        .items
        .iter()
        .find(|i| i.category_id == BONDS)
        .unwrap();
    assert_eq!(bonds.fact.dec_balance, Money::ZERO);
    assert_eq!(bonds.forecast.start_balance, Money::ZERO);
}

#[test]
fn plan_is_taken_in_december_of_the_viewed_year() {
    let mut data = base();
    // С октября «Облигации» — 10 % вместо 8 %; доходов в октябре–декабре нет.
    data.savings_rates.push(SavingsRateEntry {
        category_id: BONDS,
        valid_from: ym("2026-10"),
        rate: BasisPoints(1000),
        fixed_amount: None,
    });
    let ledger = Ledger::new(&data).unwrap();
    let overview = ledger.savings_overview(2026).unwrap();
    let bonds = overview
        .items
        .iter()
        .find(|i| i.category_id == BONDS)
        .unwrap();
    assert_eq!(bonds.plan_rate, BasisPoints(1000));
    // Средний доход года — 100 000 ₽ (три месяца с доходом), процент берётся на декабрь.
    assert_eq!(bonds.plan, rub(10_000));
    // В декабре дохода нет: месячный план — только фиксированная сумма «Отпуска».
    assert_eq!(overview.month_plan, rub(5_000));
}

#[test]
fn overview_items_match_the_standalone_forecast() {
    let mut data = base();
    deposit(&mut data, "2026-01", BONDS, 8_000);
    deposit(&mut data, "2026-02", CUSHION, 3_000);
    let ledger = Ledger::new(&data).unwrap();
    let overview = ledger.savings_overview(2026).unwrap();
    for item in &overview.items {
        let forecast = ledger
            .accumulation_forecast(item.category_id, 2026, ym("2026-12"))
            .unwrap();
        assert_eq!(item.forecast, forecast);
    }
}

#[test]
fn effective_rate_is_rate_after_tax_in_basis_points() {
    let data = base();
    let overview = Ledger::new(&data).unwrap().savings_overview(2026).unwrap();
    let rate = |c: CategoryId| {
        overview
            .items
            .iter()
            .find(|i| i.category_id == c)
            .unwrap()
            .effective_rate
    };
    // 16 % × (1 − 13 %) = 13,92 %.
    assert_eq!(rate(BONDS), BasisPoints(1392));
    assert_eq!(rate(CUSHION), BasisPoints(0));
}

#[test]
fn scenario_a_uses_the_plan_of_each_accumulation() {
    let data = base();
    let overview = Ledger::new(&data).unwrap().savings_overview(2026).unwrap();
    let a = |c: CategoryId| {
        overview
            .items
            .iter()
            .find(|i| i.category_id == c)
            .unwrap()
            .forecast
            .scenarios[0]
            .contribution
    };
    // Средний доход года — 100 000 ₽ (три месяца с доходом).
    assert_eq!(a(CUSHION), rub(4_000));
    assert_eq!(a(TRIP), rub(5_000));
    assert_eq!(a(BONDS), rub(8_000));
}

//! Краевые случаи расчётов на минимальных данных.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::calc::{Corridor, Ledger, LimitLevel};
use planning_budget_core::{
    BasisPoints, Category, CategoryId, CategoryKind, CoreError, DataSet, Income, IncomeId,
    IncomeStatus, LimitEntry, Money, SavingsParams, SavingsRateEntry, Settings, Transaction, TxId,
    TxStatus, YearMonth,
};

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

const FOOD: CategoryId = CategoryId(1);
const SAVE: CategoryId = CategoryId(2);

fn base() -> DataSet {
    let category = |id, name: &str, kind| Category {
        id,
        name: name.to_owned(),
        kind,
        color: "#000000".to_owned(),
        sort_order: 0,
        note: None,
        archived: false,
    };
    DataSet {
        settings: Settings {
            savings_min: BasisPoints(1300),
            savings_norm: BasisPoints(1400),
            savings_max: BasisPoints(1500),
            weeks_per_month: 4,
        },
        categories: vec![
            category(FOOD, "Продукты", CategoryKind::Mandatory),
            category(SAVE, "Сбережения", CategoryKind::Savings),
        ],
        limits: vec![
            LimitEntry {
                category_id: FOOD,
                valid_from: ym("2026-01"),
                amount: Some(rub(1000)),
            },
            LimitEntry {
                category_id: FOOD,
                valid_from: ym("2026-03"),
                amount: Some(rub(2000)),
            },
        ],
        savings_rates: vec![SavingsRateEntry {
            category_id: SAVE,
            valid_from: ym("2026-01"),
            rate: BasisPoints(1400),
            fixed_amount: None,
        }],
        savings_overrides: BTreeMap::new(),
        transactions: Vec::new(),
        incomes: Vec::new(),
        debts: Vec::new(),
        debt_payments: Vec::new(),
        locked_plans: BTreeMap::new(),
        savings_params: BTreeMap::from([(
            SAVE,
            SavingsParams {
                annual_rate: BasisPoints(1600),
                tax: BasisPoints(1300),
                initial_balance: Money::ZERO,
                initial_month: ym("2026-01"),
            },
        )]),
    }
}

fn income(data: &mut DataSet, month: &str, amount: Money) {
    let id = IncomeId(i64::try_from(data.incomes.len()).unwrap());
    data.incomes.push(Income {
        id,
        month: ym(month),
        source_name: "Зарплата".to_owned(),
        amount,
        status: IncomeStatus::Received,
    });
}

fn spend(data: &mut DataSet, month: &str, category: CategoryId, amount: Money, status: TxStatus) {
    let id = TxId(i64::try_from(data.transactions.len()).unwrap());
    data.transactions.push(Transaction {
        id,
        month: ym(month),
        category_id: category,
        title: "Запись".to_owned(),
        amount,
        status,
        planned_amount: None,
    });
}

#[test]
fn empty_month_has_no_rates_and_no_income_corridor() {
    let data = base();
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-05"))
        .unwrap();
    assert_eq!(s.income, Money::ZERO);
    assert_eq!(s.savings_rate, None);
    assert_eq!(s.unspent_rate, None);
    assert_eq!(s.corridor, Corridor::NoIncome);
    assert_eq!(s.per_week, Money::ZERO);
}

#[test]
fn year_without_income_divides_by_one_month() {
    let data = base();
    let y = Ledger::new(&data)
        .unwrap()
        .year_summary(2026, ym("2026-12"))
        .unwrap();
    assert_eq!(y.months_with_data, 1);
    assert_eq!(y.avg_income, Money::ZERO);
}

#[test]
fn corridor_boundaries_are_inclusive() {
    let cases = [
        (1299, Corridor::Below),
        (1300, Corridor::Within),
        (1500, Corridor::Within),
        (1501, Corridor::Above),
    ];
    for (savings_rub, expected) in cases {
        let mut data = base();
        income(&mut data, "2026-02", rub(10_000));
        spend(&mut data, "2026-02", SAVE, rub(savings_rub), TxStatus::Paid);
        let s = Ledger::new(&data)
            .unwrap()
            .month_summary(ym("2026-02"))
            .unwrap();
        assert_eq!(s.corridor, expected, "savings: {savings_rub}");
    }
}

#[test]
fn planned_spending_counts_and_free_can_be_negative() {
    let mut data = base();
    income(&mut data, "2026-02", rub(1000));
    spend(&mut data, "2026-02", FOOD, rub(1500), TxStatus::Planned);
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-02"))
        .unwrap();
    assert_eq!(s.expenses, rub(1500));
    assert_eq!(s.free, rub(-500));
    assert_eq!(s.per_week, rub(-125));
    assert_eq!(s.by_status.planned, rub(1500));
}

#[test]
fn savings_plan_uses_month_override_over_category_rate() {
    let mut data = base();
    income(&mut data, "2026-02", rub(10_000));
    income(&mut data, "2026-03", rub(10_000));
    data.savings_overrides
        .insert((ym("2026-03"), SAVE), BasisPoints(2000));
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(ledger.savings_plan(ym("2026-02")).unwrap(), rub(1400));
    assert_eq!(ledger.savings_plan(ym("2026-03")).unwrap(), rub(2000));
    assert_eq!(ledger.limit(ym("2026-03"), SAVE).unwrap(), Some(rub(2000)));
}

const RESERVE: CategoryId = CategoryId(3);

fn with_reserve(rate: i32) -> DataSet {
    let mut data = base();
    data.categories.push(Category {
        id: RESERVE,
        name: "Подушка".to_owned(),
        kind: CategoryKind::Savings,
        color: "#000000".to_owned(),
        sort_order: 0,
        note: None,
        archived: false,
    });
    data.savings_rates.push(SavingsRateEntry {
        category_id: RESERVE,
        valid_from: ym("2026-01"),
        rate: BasisPoints(rate),
        fixed_amount: None,
    });
    data
}

#[test]
fn savings_plan_is_sum_of_per_category_plans() {
    let mut data = with_reserve(500);
    data.savings_rates[0].rate = BasisPoints(1300);
    income(&mut data, "2026-02", rub(100_000));
    spend(&mut data, "2026-02", SAVE, rub(8000), TxStatus::Paid);
    spend(&mut data, "2026-02", RESERVE, rub(2000), TxStatus::Paid);
    let ledger = Ledger::new(&data).unwrap();
    let feb = ym("2026-02");
    assert_eq!(ledger.limit(feb, SAVE).unwrap(), Some(rub(13_000)));
    assert_eq!(ledger.limit(feb, RESERVE).unwrap(), Some(rub(5000)));
    assert_eq!(ledger.savings_plan(feb).unwrap(), rub(18_000));
    assert_eq!(ledger.savings_plan_rate(feb), BasisPoints(1800));

    let summary = ledger.month_summary(feb).unwrap();
    assert_eq!(summary.savings_plan, rub(18_000));
    assert_eq!(summary.savings_gap, rub(-8000));
    let limits = ledger.month_limits(feb).unwrap();
    let row = |id| limits.rows.iter().find(|r| r.category_id == id).unwrap();
    assert_eq!(row(SAVE).remaining, Some(rub(5000)));
    assert_eq!(row(RESERVE).remaining, Some(rub(3000)));
}

#[test]
fn savings_override_applies_to_its_category_only() {
    let mut data = with_reserve(500);
    data.savings_overrides
        .insert((ym("2026-02"), RESERVE), BasisPoints(0));
    income(&mut data, "2026-02", rub(10_000));
    let ledger = Ledger::new(&data).unwrap();
    let feb = ym("2026-02");
    assert_eq!(ledger.limit(feb, SAVE).unwrap(), Some(rub(1400)));
    assert_eq!(ledger.limit(feb, RESERVE).unwrap(), Some(Money::ZERO));
    assert_eq!(ledger.savings_plan(feb).unwrap(), rub(1400));
}

#[test]
fn savings_rate_follows_history_and_is_zero_before_first_entry() {
    let mut data = base();
    data.savings_rates.push(SavingsRateEntry {
        category_id: SAVE,
        valid_from: ym("2026-04"),
        rate: BasisPoints(1500),
        fixed_amount: None,
    });
    for month in ["2025-12", "2026-03", "2026-04"] {
        income(&mut data, month, rub(10_000));
    }
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(ledger.savings_plan(ym("2025-12")).unwrap(), Money::ZERO);
    assert_eq!(ledger.savings_plan(ym("2026-03")).unwrap(), rub(1400));
    assert_eq!(ledger.savings_plan(ym("2026-04")).unwrap(), rub(1500));
}

#[test]
fn limit_follows_history_and_is_absent_before_first_entry() {
    let data = base();
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(ledger.limit(ym("2025-12"), FOOD).unwrap(), None);
    assert_eq!(ledger.limit(ym("2026-02"), FOOD).unwrap(), Some(rub(1000)));
    assert_eq!(ledger.limit(ym("2026-03"), FOOD).unwrap(), Some(rub(2000)));
    assert_eq!(ledger.limit(ym("2027-01"), FOOD).unwrap(), Some(rub(2000)));
}

#[test]
fn limit_levels_switch_at_85_and_100_percent() {
    let cases = [
        (849, LimitLevel::Ok),
        (850, LimitLevel::Warn),
        (1000, LimitLevel::Warn),
        (1001, LimitLevel::Over),
    ];
    for (fact_rub, expected) in cases {
        let mut data = base();
        spend(&mut data, "2026-02", FOOD, rub(fact_rub), TxStatus::Paid);
        let limits = Ledger::new(&data)
            .unwrap()
            .month_limits(ym("2026-02"))
            .unwrap();
        let row = limits.rows.iter().find(|r| r.category_id == FOOD).unwrap();
        assert_eq!(row.level, Some(expected), "fact: {fact_rub}");
    }
}

#[test]
fn zero_limit_with_spending_is_over_with_full_usage() {
    let mut data = base();
    data.limits[0].amount = Some(Money::ZERO);
    spend(&mut data, "2026-02", FOOD, rub(1), TxStatus::Paid);
    let limits = Ledger::new(&data)
        .unwrap()
        .month_limits(ym("2026-02"))
        .unwrap();
    let row = limits.rows.iter().find(|r| r.category_id == FOOD).unwrap();
    assert_eq!(row.usage, Some(1.0));
    assert_eq!(row.level, Some(LimitLevel::Over));
}

#[test]
fn savings_row_has_no_level() {
    let mut data = base();
    income(&mut data, "2026-02", rub(10_000));
    spend(&mut data, "2026-02", SAVE, rub(5000), TxStatus::Paid);
    let limits = Ledger::new(&data)
        .unwrap()
        .month_limits(ym("2026-02"))
        .unwrap();
    let row = limits.rows.iter().find(|r| r.category_id == SAVE).unwrap();
    assert_eq!(row.limit, Some(rub(1400)));
    assert_eq!(row.level, None);
    assert_eq!(limits.limits_total, rub(1000));
}

#[test]
fn unknown_category_is_an_error() {
    let mut data = base();
    spend(&mut data, "2026-02", CategoryId(99), rub(1), TxStatus::Paid);
    assert_eq!(
        Ledger::new(&data).unwrap_err(),
        CoreError::CategoryNotFound(99)
    );
}

#[test]
fn accumulation_starts_from_initial_month_with_initial_balance() {
    let mut data = base();
    let params = data.savings_params.get_mut(&SAVE).unwrap();
    params.initial_month = ym("2026-11");
    params.initial_balance = rub(100_000);
    spend(&mut data, "2026-11", SAVE, rub(10_000), TxStatus::Paid);
    let fact = Ledger::new(&data)
        .unwrap()
        .accumulation_fact(SAVE, 2026)
        .unwrap();
    assert_eq!(fact.months.len(), 2);
    assert_eq!(fact.months[0].month, ym("2026-11"));
    assert_eq!(fact.months[0].deposited, rub(110_000));
    assert!(fact.months[0].coupon_income > Money::ZERO);
}

#[test]
fn accumulation_before_initial_month_has_no_balance() {
    let mut data = base();
    let params = data.savings_params.get_mut(&SAVE).unwrap();
    params.initial_month = ym("2027-03");
    params.initial_balance = rub(5);
    let fact = Ledger::new(&data)
        .unwrap()
        .accumulation_fact(SAVE, 2026)
        .unwrap();
    assert!(fact.months.is_empty());
    // Накопление ещё не началось: стартовые 5 ₽ появятся только в `initial_month`.
    assert_eq!(fact.dec_balance, Money::ZERO);
}

#[test]
fn zero_weeks_per_month_is_rejected() {
    let mut data = base();
    data.settings.weeks_per_month = 0;
    assert!(matches!(Ledger::new(&data), Err(CoreError::Settings(_))));
}

#[test]
fn accumulation_balance_carries_over_into_next_year() {
    let mut data = base();
    spend(&mut data, "2026-06", SAVE, rub(10_000), TxStatus::Paid);
    spend(&mut data, "2027-02", SAVE, rub(5000), TxStatus::Paid);
    let ledger = Ledger::new(&data).unwrap();
    let dec_2026 = ledger.accumulation_fact(SAVE, 2026).unwrap().dec_balance;
    let next = ledger.accumulation_fact(SAVE, 2027).unwrap();
    assert_eq!(next.months.len(), 12);
    assert_eq!(next.months[0].month, ym("2027-01"));
    assert!(next.months[0].balance > dec_2026);
    assert_eq!(next.months[1].deposited, rub(15_000));
}

#[test]
fn zero_limit_without_spending_is_ok() {
    let mut data = base();
    data.limits[0].amount = Some(Money::ZERO);
    let limits = Ledger::new(&data)
        .unwrap()
        .month_limits(ym("2026-02"))
        .unwrap();
    let row = limits.rows.iter().find(|r| r.category_id == FOOD).unwrap();
    assert_eq!(row.usage, Some(0.0));
    assert_eq!(row.level, Some(LimitLevel::Ok));
}

#[test]
fn archived_category_limit_is_excluded_from_totals() {
    let mut data = base();
    income(&mut data, "2026-02", rub(10_000));
    spend(&mut data, "2026-02", FOOD, rub(1500), TxStatus::Paid);
    data.categories[0].archived = true;
    let ledger = Ledger::new(&data).unwrap();

    let limits = ledger.month_limits(ym("2026-02")).unwrap();
    assert!(limits.rows.iter().any(|r| r.category_id == FOOD));
    assert_eq!(limits.limits_total, Money::ZERO);

    let balance = ledger.budget_balance(2026, ym("2026-02")).unwrap();
    assert_eq!(balance.limits_sum, Money::ZERO);
    assert_eq!(balance.economy, Money::ZERO);
}

#[test]
fn below_corridor_always_has_positive_top_up() {
    let mut data = base();
    income(&mut data, "2026-02", Money::from_kopecks(10_001));
    spend(
        &mut data,
        "2026-02",
        SAVE,
        Money::from_kopecks(1300),
        TxStatus::Paid,
    );
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-02"))
        .unwrap();
    assert_eq!(s.corridor, Corridor::Below);
    assert_eq!(s.top_up_to_min, Money::from_kopecks(1));
}

#[test]
fn top_up_is_zero_inside_corridor() {
    let mut data = base();
    income(&mut data, "2026-02", rub(10_000));
    spend(&mut data, "2026-02", SAVE, rub(1300), TxStatus::Paid);
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-02"))
        .unwrap();
    assert_eq!(s.top_up_to_min, Money::ZERO);
    assert_eq!(s.top_up_to_norm, rub(100));
}

#[test]
fn invalid_year_is_an_error_everywhere() {
    let data = base();
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(
        ledger.year_summary(1969, ym("2026-02")).unwrap_err(),
        CoreError::Month
    );
    assert_eq!(
        ledger
            .category_year_total(0, ym("2026-02"), FOOD)
            .unwrap_err(),
        CoreError::Month
    );
    assert_eq!(
        ledger.budget_balance(1969, ym("2026-02")).unwrap_err(),
        CoreError::Month
    );
    assert_eq!(
        ledger.accumulation_fact(SAVE, 1969).unwrap_err(),
        CoreError::Month
    );
}

#[test]
fn year_free_equals_sum_of_month_free() {
    let mut data = base();
    income(&mut data, "2026-02", rub(10_000));
    income(&mut data, "2026-03", rub(8000));
    spend(&mut data, "2026-02", FOOD, rub(3000), TxStatus::Paid);
    spend(&mut data, "2026-03", SAVE, rub(1000), TxStatus::Planned);
    let ledger = Ledger::new(&data).unwrap();
    let year = ledger.year_summary(2026, ym("2026-12")).unwrap();
    assert_eq!(year.free, rub(14_000));
    assert_eq!(
        ledger.month_summary(ym("2026-12")).unwrap().free_cum,
        year.free
    );
}

#[test]
fn year_summary_ignores_months_after_current() {
    let mut data = base();
    income(&mut data, "2026-09", rub(10_000));
    income(&mut data, "2026-10", rub(20_000));
    income(&mut data, "2026-11", rub(30_000));
    spend(&mut data, "2026-10", FOOD, rub(500), TxStatus::Paid);
    spend(&mut data, "2026-11", FOOD, rub(700), TxStatus::Planned);
    let ledger = Ledger::new(&data).unwrap();

    let y = ledger.year_summary(2026, ym("2026-10")).unwrap();
    assert_eq!(y.months_with_data, 2);
    assert_eq!(y.income, rub(30_000));
    assert_eq!(y.avg_income, rub(15_000));
    assert_eq!(y.expenses, rub(500));
    assert_eq!(
        ledger
            .category_year_total(2026, ym("2026-10"), FOOD)
            .unwrap(),
        rub(500)
    );
}

/// `Ledger::new` копит серии подряд идущих трат с одним ключом: порядок записей может
/// ускорять расчёт, но не должен менять результат.
#[test]
fn ledger_totals_do_not_depend_on_transaction_order() {
    const SEED: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/seed-2026.json"
    ));
    let sorted = planning_budget_core::load_seed(SEED).unwrap();
    let mut shuffled = sorted.clone();
    // Перемешивание без зависимостей: шаг взаимно прост с длиной, обход даёт перестановку.
    let n = shuffled.transactions.len();
    let step = (1..n).rev().find(|s| gcd(*s, n) == 1).unwrap();
    shuffled.transactions = (0..n)
        .map(|i| sorted.transactions[(i * step) % n].clone())
        .collect();
    assert_ne!(sorted.transactions, shuffled.transactions);

    let today = ym("2026-09");
    let a = Ledger::new(&sorted).unwrap();
    let b = Ledger::new(&shuffled).unwrap();
    assert_eq!(
        a.year_summary(2026, today).unwrap(),
        b.year_summary(2026, today).unwrap()
    );
    for month in YearMonth::months_of_year(2026).unwrap() {
        assert_eq!(
            a.month_summary(month).unwrap(),
            b.month_summary(month).unwrap()
        );
    }
    assert_eq!(
        a.category_year_rows(2026, today).unwrap(),
        b.category_year_rows(2026, today).unwrap()
    );
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[test]
fn status_shares_are_basis_points_of_the_total_and_zero_when_empty() {
    let mut data = base();
    spend(&mut data, "2026-02", FOOD, rub(300), TxStatus::Paid);
    spend(&mut data, "2026-02", FOOD, rub(100), TxStatus::Planned);
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-02"))
        .unwrap();
    let shares = s.by_status.shares_bp();
    assert_eq!((shares.paid, shares.planned), (7_500, 2_500));
    assert_eq!((shares.debt, shares.unplanned), (0, 0));

    let empty = Ledger::new(&base())
        .unwrap()
        .month_summary(ym("2026-02"))
        .unwrap();
    assert_eq!(empty.by_status.shares_bp(), Default::default());
}

#[test]
fn limit_row_reports_percent_split_between_paid_and_planned_and_over_count() {
    let mut data = base();
    // Лимит «Продуктов» в феврале — 1 000 ₽.
    spend(&mut data, "2026-02", FOOD, rub(600), TxStatus::Paid);
    spend(&mut data, "2026-02", FOOD, rub(500), TxStatus::Planned);
    let limits = Ledger::new(&data)
        .unwrap()
        .month_limits(ym("2026-02"))
        .unwrap();
    let food = limits.rows.iter().find(|r| r.category_id == FOOD).unwrap();
    assert_eq!(food.usage_percent, Some(110));
    assert_eq!(food.level, Some(LimitLevel::Over));
    assert!((food.paid_usage.unwrap() - 0.6).abs() < 1e-9);
    assert!((food.planned_usage.unwrap() - 0.5).abs() < 1e-9);
    assert_eq!(limits.over_count, 1);
}

#[test]
fn zero_limit_has_full_percent_and_no_split() {
    let mut data = base();
    data.limits.push(LimitEntry {
        category_id: FOOD,
        valid_from: ym("2026-05"),
        amount: Some(Money::ZERO),
    });
    spend(&mut data, "2026-05", FOOD, rub(10), TxStatus::Paid);
    let limits = Ledger::new(&data)
        .unwrap()
        .month_limits(ym("2026-05"))
        .unwrap();
    let food = limits.rows.iter().find(|r| r.category_id == FOOD).unwrap();
    assert_eq!(food.usage_percent, Some(100));
    assert_eq!((food.paid_usage, food.planned_usage), (None, None));
}

#[test]
fn savings_rate_bp_matches_the_rate_and_is_absent_without_income() {
    let mut data = base();
    income(&mut data, "2026-02", rub(10_000));
    spend(&mut data, "2026-02", SAVE, rub(1_234), TxStatus::Paid);
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(
        ledger.month_summary(ym("2026-02")).unwrap().savings_rate_bp,
        Some(1_234)
    );
    assert_eq!(
        ledger.month_summary(ym("2026-03")).unwrap().savings_rate_bp,
        None
    );
}

#[test]
fn expenses_delta_needs_two_months_of_data_and_a_positive_average() {
    let mut data = base();
    income(&mut data, "2026-01", rub(10_000));
    spend(&mut data, "2026-01", FOOD, rub(1_000), TxStatus::Paid);
    let one_month = Ledger::new(&data)
        .unwrap()
        .year_summary(2026, ym("2026-01"))
        .unwrap();
    assert_eq!(one_month.expenses_delta_percent(rub(1_200)), None);

    income(&mut data, "2026-02", rub(10_000));
    spend(&mut data, "2026-02", FOOD, rub(500), TxStatus::Paid);
    let two_months = Ledger::new(&data)
        .unwrap()
        .year_summary(2026, ym("2026-02"))
        .unwrap();
    // Среднее за два месяца — 750 ₽: 600 ₽ ниже среднего на 20 %, 900 ₽ выше на 20 %.
    assert_eq!(two_months.expenses_delta_percent(rub(600)), Some(-20));
    assert_eq!(two_months.expenses_delta_percent(rub(900)), Some(20));

    let no_expenses = Ledger::new(&base())
        .unwrap()
        .year_summary(2026, ym("2026-02"))
        .unwrap();
    assert_eq!(no_expenses.expenses_delta_percent(rub(100)), None);
}

#[test]
fn plan_rate_comes_from_history_and_off_norm_flag_compares_with_the_norm() {
    let mut data = base();
    data.savings_rates[0].valid_from = ym("2026-03");
    let ledger = Ledger::new(&data).unwrap();
    let rate_of = |month: &str| {
        ledger
            .month_limits(ym(month))
            .unwrap()
            .rows
            .iter()
            .find(|r| r.category_id == SAVE)
            .unwrap()
            .plan_rate_bp
    };
    assert_eq!(rate_of("2026-02"), None);
    assert_eq!(rate_of("2026-03"), Some(1400));
    // Продукты — не накопление: процента у строки нет.
    let food = ledger.month_limits(ym("2026-03")).unwrap();
    let food = food.rows.iter().find(|r| r.category_id == FOOD).unwrap();
    assert_eq!(food.plan_rate_bp, None);

    // План 14 % равен норме 14 %: расхождения нет; 15 % — есть; без плана — нет.
    let flag = |data: &DataSet, month: &str| {
        Ledger::new(data)
            .unwrap()
            .month_summary(ym(month))
            .unwrap()
            .savings_plan_off_norm
    };
    assert!(!flag(&data, "2026-03"));
    assert!(!flag(&data, "2026-02"));
    data.savings_rates[0].rate = BasisPoints(1500);
    assert!(flag(&data, "2026-03"));
}

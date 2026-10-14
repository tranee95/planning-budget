//! Накопленное: итог за весь период и взносы по месяцам.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::calc::Ledger;
use planning_budget_core::{
    BasisPoints, Category, CategoryId, CategoryKind, DataSet, Money, Settings, Transaction, TxId,
    TxStatus, YearMonth,
};

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

const CUSHION: CategoryId = CategoryId(1);
const TRIP: CategoryId = CategoryId(2);
const FOOD: CategoryId = CategoryId(3);

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
            category(CUSHION, "Подушка", CategoryKind::Savings),
            category(TRIP, "Отпуск", CategoryKind::Savings),
            category(FOOD, "Продукты", CategoryKind::Mandatory),
        ],
        limits: Vec::new(),
        savings_rates: Vec::new(),
        savings_overrides: BTreeMap::new(),
        transactions: Vec::new(),
        incomes: Vec::new(),
        debts: Vec::new(),
        debt_payments: Vec::new(),
        locked_plans: BTreeMap::new(),
        savings_params: BTreeMap::new(),
    }
}

fn add(data: &mut DataSet, month: &str, category: CategoryId, amount: i64, status: TxStatus) {
    let id = TxId(i64::try_from(data.transactions.len()).unwrap() + 1);
    data.transactions.push(Transaction {
        id,
        month: ym(month),
        category_id: category,
        title: "Накопление".to_owned(),
        amount: rub(amount),
        status,
        planned_amount: None,
    });
}

#[test]
fn no_deposits_gives_an_empty_progress() {
    let data = base();
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-10"))
        .unwrap();
    assert!(p.months.is_empty());
    assert_eq!(p.total, Money::ZERO);
}

#[test]
fn total_covers_the_whole_period_and_all_accumulations() {
    let mut data = base();
    add(&mut data, "2025-11", CUSHION, 10_000, TxStatus::Paid);
    add(&mut data, "2025-12", CUSHION, 10_000, TxStatus::Paid);
    add(&mut data, "2025-12", TRIP, 5_000, TxStatus::Paid);
    add(&mut data, "2026-02", CUSHION, 20_000, TxStatus::Paid);
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-03"))
        .unwrap();
    assert_eq!(p.total, rub(45_000));
}

#[test]
fn months_run_without_gaps_from_the_first_deposit_to_the_current_month() {
    let mut data = base();
    add(&mut data, "2025-12", CUSHION, 10_000, TxStatus::Paid);
    add(&mut data, "2026-02", CUSHION, 20_000, TxStatus::Paid);
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-03"))
        .unwrap();
    let rows: Vec<(String, Money, Money)> = p
        .months
        .iter()
        .map(|m| (m.month.to_string(), m.deposited, m.cumulative))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("2025-12".to_owned(), rub(10_000), rub(10_000)),
            ("2026-01".to_owned(), Money::ZERO, rub(10_000)),
            ("2026-02".to_owned(), rub(20_000), rub(30_000)),
            ("2026-03".to_owned(), Money::ZERO, rub(30_000)),
        ]
    );
}

#[test]
fn other_categories_and_planned_rows_are_not_deposits() {
    let mut data = base();
    add(&mut data, "2026-03", FOOD, 7_000, TxStatus::Paid);
    add(&mut data, "2026-03", CUSHION, 3_000, TxStatus::Planned);
    add(&mut data, "2026-03", CUSHION, 1_000, TxStatus::Paid);
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-03"))
        .unwrap();
    assert_eq!(p.total, rub(1_000));
}

#[test]
fn deposits_after_the_current_month_are_not_counted() {
    let mut data = base();
    add(&mut data, "2026-03", CUSHION, 1_000, TxStatus::Paid);
    add(&mut data, "2026-05", CUSHION, 9_000, TxStatus::Paid);
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-03"))
        .unwrap();
    assert_eq!(p.total, rub(1_000));
}

#[test]
fn every_accumulation_has_its_own_amounts_per_month_including_closed_ones() {
    let mut data = base();
    // «Подушка» — две суммы в сентябре и одна в октябре; «Отпуск» (закрытая цель) — только в августе.
    add(&mut data, "2026-08", TRIP, 20_000, TxStatus::Paid);
    add(&mut data, "2026-09", CUSHION, 10_000, TxStatus::Paid);
    add(&mut data, "2026-09", CUSHION, 5_000, TxStatus::Paid);
    add(&mut data, "2026-10", CUSHION, 7_000, TxStatus::Paid);
    data.categories[1].archived = true;
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-10"))
        .unwrap();
    assert_eq!(p.months.len(), 3);
    // Порядок — как у категорий; пустых накоплений и обычных категорий в списке нет.
    let ids: Vec<CategoryId> = p.categories.iter().map(|c| c.category_id).collect();
    assert_eq!(ids, vec![CUSHION, TRIP]);
    let cushion = &p.categories[0];
    assert_eq!(cushion.amounts, vec![Money::ZERO, rub(15_000), rub(7_000)]);
    assert_eq!(cushion.total, rub(22_000));
    let trip = &p.categories[1];
    assert_eq!(trip.amounts, vec![rub(20_000), Money::ZERO, Money::ZERO]);
    assert_eq!(trip.total, rub(20_000));
    // Сумма по накоплениям в каждом месяце совпадает с итогом месяца.
    for (i, month) in p.months.iter().enumerate() {
        let sum = p
            .categories
            .iter()
            .fold(Money::ZERO, |acc, c| acc.checked_add(c.amounts[i]).unwrap());
        assert_eq!(sum, month.deposited);
    }
}

#[test]
fn accumulation_without_any_amount_is_not_listed() {
    let mut data = base();
    add(&mut data, "2026-10", CUSHION, 3_000, TxStatus::Paid);
    add(&mut data, "2026-10", TRIP, 1_000, TxStatus::Planned);
    let p = Ledger::new(&data)
        .unwrap()
        .savings_progress(ym("2026-10"))
        .unwrap();
    assert_eq!(p.categories.len(), 1);
    assert_eq!(p.categories[0].category_id, CUSHION);
}

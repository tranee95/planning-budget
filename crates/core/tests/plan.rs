//! План месяца: «Не распределено», план → факт, фиксация и копирование.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::calc::{Ledger, MonthPlan};
use planning_budget_core::{
    BasisPoints, Category, CategoryId, CategoryKind, DataSet, Debt, DebtId, DebtPayment,
    DebtPaymentStatus, Income, IncomeId, IncomeStatus, LockedPlan, Money, SavingsRateEntry,
    Settings, Transaction, TxId, TxStatus, YearMonth,
};

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

const FOOD: CategoryId = CategoryId(1);
const FUN: CategoryId = CategoryId(2);
const SAVE: CategoryId = CategoryId(3);
const OLD: CategoryId = CategoryId(4);

fn base() -> DataSet {
    let category = |id, name: &str, kind, archived| Category {
        id,
        name: name.to_owned(),
        kind,
        color: "#000000".to_owned(),
        sort_order: 0,
        note: None,
        archived,
    };
    DataSet {
        settings: Settings {
            savings_min: BasisPoints(1300),
            savings_norm: BasisPoints(1400),
            savings_max: BasisPoints(1500),
            weeks_per_month: 4,
        },
        categories: vec![
            category(FOOD, "Продукты", CategoryKind::Mandatory, false),
            category(FUN, "Развлечения", CategoryKind::Wants, false),
            category(SAVE, "Сбережения", CategoryKind::Savings, false),
            category(OLD, "Старое", CategoryKind::Wants, true),
        ],
        limits: Vec::new(),
        savings_rates: vec![SavingsRateEntry {
            category_id: SAVE,
            valid_from: ym("2026-01"),
            rate: BasisPoints(1400),
            fixed_amount: None,
        }],
        savings_overrides: BTreeMap::new(),
        transactions: Vec::new(),
        incomes: vec![Income {
            id: IncomeId(1),
            month: ym("2026-10"),
            source_name: "Зарплата".to_owned(),
            amount: rub(100_000),
            status: IncomeStatus::Received,
        }],
        debts: Vec::new(),
        debt_payments: Vec::new(),
        locked_plans: BTreeMap::new(),
        savings_params: BTreeMap::new(),
    }
}

fn spend(
    data: &mut DataSet,
    month: &str,
    category: CategoryId,
    amount: i64,
    status: TxStatus,
    planned: Option<i64>,
) -> TxId {
    let id = TxId(i64::try_from(data.transactions.len()).unwrap() + 1);
    data.transactions.push(Transaction {
        id,
        month: ym(month),
        category_id: category,
        title: format!("Запись {}", id.0),
        amount: rub(amount),
        status,
        planned_amount: planned.map(rub),
    });
    id
}

fn plan(data: &DataSet, month: &str) -> MonthPlan {
    Ledger::new(data).unwrap().month_plan(ym(month)).unwrap()
}

fn row(plan: &MonthPlan, category: CategoryId) -> planning_budget_core::calc::PlanRow {
    *plan
        .rows
        .iter()
        .find(|r| r.category_id == category)
        .unwrap()
}

/// Октябрь: доход 100 000 ₽, план трат 55 000 ₽ (в том числе уже оплаченное и «Долг»), «Незапланировано»
/// 3 000 ₽ в план не входит, накопление 14 % = 14 000 ₽, погашение долга 10 000 ₽.
fn october() -> DataSet {
    let mut data = base();
    spend(&mut data, "2026-10", FOOD, 30_000, TxStatus::Paid, None);
    spend(&mut data, "2026-10", FOOD, 5_000, TxStatus::Planned, None);
    spend(&mut data, "2026-10", FUN, 20_000, TxStatus::Debt, None);
    spend(&mut data, "2026-10", FUN, 3_000, TxStatus::Unplanned, None);
    data.debts.push(Debt {
        id: DebtId(1),
        lender: "Карта".to_owned(),
        amount: rub(10_000),
        taken_month: ym("2026-09"),
        category_id: None,
    });
    data.debt_payments.push(DebtPayment {
        debt_id: DebtId(1),
        month: ym("2026-10"),
        amount: rub(10_000),
        status: DebtPaymentStatus::Planned,
    });
    data
}

#[test]
fn unallocated_is_income_minus_plan_expenses_savings_and_repayments() {
    let p = plan(&october(), "2026-10");
    assert!(!p.locked);
    assert_eq!(p.income, rub(100_000));
    assert_eq!(p.plan_expenses, rub(55_000));
    assert_eq!(p.plan_savings, rub(14_000));
    assert_eq!(p.plan_repayments, rub(10_000));
    assert_eq!(p.unallocated, rub(21_000));
    assert_eq!(p.unplanned, rub(3_000));
}

#[test]
fn plan_to_fact_rows_show_deviation_per_category() {
    let p = plan(&october(), "2026-10");
    let food = row(&p, FOOD);
    assert_eq!(
        (food.plan, food.fact, food.deviation),
        (rub(35_000), rub(35_000), Money::ZERO)
    );
    // «Развлечения»: план 20 000 ₽ (долг), факт 23 000 ₽ с незапланированной тратой.
    let fun = row(&p, FUN);
    assert_eq!(
        (fun.plan, fun.fact, fun.deviation),
        (rub(20_000), rub(23_000), rub(3_000))
    );
    let save = row(&p, SAVE);
    assert_eq!((save.plan, save.fact), (rub(14_000), Money::ZERO));
}

#[test]
fn archived_category_without_plan_or_fact_has_no_row() {
    let p = plan(&october(), "2026-10");
    assert!(p.rows.iter().all(|r| r.category_id != OLD));
}

#[test]
fn plan_over_income_makes_unallocated_negative() {
    let mut data = october();
    spend(&mut data, "2026-10", FUN, 40_000, TxStatus::Planned, None);
    assert_eq!(plan(&data, "2026-10").unallocated, rub(-19_000));
}

#[test]
fn month_without_plan_has_all_income_unallocated() {
    let data = base();
    let p = plan(&data, "2026-10");
    assert_eq!(p.plan_expenses, Money::ZERO);
    assert_eq!(
        p.unallocated,
        rub(100_000).checked_sub(p.plan_savings).unwrap()
    );
}

#[test]
fn lock_snapshot_freezes_planned_statuses_savings_and_repayments() {
    let data = october();
    let snapshot = Ledger::new(&data)
        .unwrap()
        .lock_snapshot(ym("2026-10"))
        .unwrap();
    let ids: Vec<i64> = snapshot
        .planned_amounts
        .iter()
        .map(|(id, _)| id.0)
        .collect();
    // Траты 1–3 (оплачено, план, долг) входят, 4 («Незапланировано») — нет.
    assert_eq!(ids, vec![1, 2, 3]);
    assert_eq!(snapshot.savings, vec![(SAVE, rub(14_000))]);
    assert_eq!(snapshot.repayments_planned, rub(10_000));
}

/// Применяет снимок к набору так, как это делает хранилище.
fn lock(data: &mut DataSet, month: &str) {
    let snapshot = Ledger::new(data).unwrap().lock_snapshot(ym(month)).unwrap();
    for (id, amount) in snapshot.planned_amounts {
        data.transactions
            .iter_mut()
            .find(|t| t.id == id)
            .unwrap()
            .planned_amount = Some(amount);
    }
    data.locked_plans.insert(
        ym(month),
        LockedPlan {
            repayments_planned: snapshot.repayments_planned,
            savings: snapshot.savings.into_iter().collect(),
        },
    );
}

#[test]
fn locked_plan_keeps_planned_sums_when_facts_change() {
    let mut data = october();
    lock(&mut data, "2026-10");
    let before = plan(&data, "2026-10");
    assert!(before.locked);
    assert_eq!(before.unallocated, rub(21_000));

    // Продукты оплачены на 2 000 ₽ дороже; новая трата после фиксации — «Незапланировано».
    data.transactions[0].amount = rub(32_000);
    spend(&mut data, "2026-10", FUN, 4_000, TxStatus::Unplanned, None);
    let after = plan(&data, "2026-10");
    assert_eq!(after.unallocated, rub(21_000), "план не двигается");
    let food = row(&after, FOOD);
    assert_eq!(
        (food.plan, food.fact, food.deviation),
        (rub(35_000), rub(37_000), rub(2_000))
    );
    assert_eq!(after.unplanned, rub(7_000));
}

#[test]
fn locked_savings_and_repayments_ignore_later_changes() {
    let mut data = october();
    lock(&mut data, "2026-10");
    data.incomes[0].amount = rub(200_000);
    data.debt_payments[0].amount = rub(99_000);
    let p = plan(&data, "2026-10");
    assert_eq!(
        p.plan_savings,
        rub(14_000),
        "снимок накопления, а не 14 % нового дохода"
    );
    assert_eq!(p.plan_repayments, rub(10_000));
}

#[test]
fn unlocked_month_follows_current_amounts() {
    let mut data = october();
    data.transactions[1].amount = rub(9_000);
    assert_eq!(plan(&data, "2026-10").plan_expenses, rub(59_000));
}

#[test]
fn copy_rows_use_statuses_when_open_and_planned_amounts_when_locked() {
    let mut data = october();
    let titles = |rows: Vec<planning_budget_core::calc::PlanCopyRow>| -> Vec<(i64, i64)> {
        rows.into_iter()
            .map(|r| (r.category_id.0, r.amount.kopecks() / 100))
            .collect()
    };
    let open = titles(Ledger::new(&data).unwrap().plan_copy_rows(ym("2026-10")));
    assert_eq!(open, vec![(1, 30_000), (1, 5_000), (2, 20_000)]);

    lock(&mut data, "2026-10");
    data.transactions[0].amount = rub(32_000);
    // Копируется то, что планировали, а не то, что вышло по факту.
    let locked = titles(Ledger::new(&data).unwrap().plan_copy_rows(ym("2026-10")));
    assert_eq!(locked, vec![(1, 30_000), (1, 5_000), (2, 20_000)]);
}

#[test]
fn copy_rows_skip_savings_categories() {
    let mut data = base();
    spend(&mut data, "2026-10", SAVE, 14_000, TxStatus::Paid, None);
    spend(&mut data, "2026-10", FOOD, 1_000, TxStatus::Planned, None);
    let rows = Ledger::new(&data).unwrap().plan_copy_rows(ym("2026-10"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].category_id, FOOD);
}

fn plan_with(income: i64, expenses: i64, savings: i64, repayments: i64) -> MonthPlan {
    MonthPlan {
        month: ym("2026-09"),
        locked: false,
        income: rub(income),
        plan_expenses: rub(expenses),
        plan_savings: rub(savings),
        plan_repayments: rub(repayments),
        unallocated: rub(income - expenses - savings - repayments),
        rows: Vec::new(),
        unplanned: Money::ZERO,
        borrowed: Money::ZERO,
        saved: Money::ZERO,
    }
}

#[test]
fn balance_tells_empty_balanced_unallocated_and_over_apart() {
    use planning_budget_core::calc::PlanBalance;

    assert_eq!(plan_with(0, 0, 0, 0).balance(), PlanBalance::Empty);
    assert_eq!(plan_with(100, 60, 30, 10).balance(), PlanBalance::Balanced);
    assert_eq!(
        plan_with(100, 60, 20, 10).balance(),
        PlanBalance::Unallocated
    );
    assert_eq!(plan_with(100, 80, 20, 10).balance(), PlanBalance::Over);
    // План без дохода — не «пусто»: он превышает доход.
    assert_eq!(plan_with(0, 10, 0, 0).balance(), PlanBalance::Over);
    // Доход без плана — всё не распределено.
    assert_eq!(plan_with(100, 0, 0, 0).balance(), PlanBalance::Unallocated);
}

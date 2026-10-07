//! Долги: учёт без двойного счёта, остаток и график погашений.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::calc::{
    Ledger, equal_parts_schedule, single_payment_schedule, validate_schedule,
};
use planning_budget_core::{
    BasisPoints, Category, CategoryId, CategoryKind, CoreError, DataSet, Debt, DebtId, DebtPayment,
    DebtPaymentStatus, Income, IncomeId, IncomeStatus, Money, Settings, Transaction, TxId,
    TxStatus, YearMonth,
};
use proptest::prelude::*;

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

const CLOTHES: CategoryId = CategoryId(1);
const DEBT: DebtId = DebtId(1);

fn base() -> DataSet {
    DataSet {
        settings: Settings {
            savings_min: BasisPoints(1300),
            savings_norm: BasisPoints(1400),
            savings_max: BasisPoints(1500),
            weeks_per_month: 4,
        },
        categories: vec![Category {
            id: CLOTHES,
            name: "Одежда".to_owned(),
            kind: CategoryKind::Wants,
            color: "#000000".to_owned(),
            sort_order: 0,
            note: None,
            archived: false,
        }],
        limits: Vec::new(),
        savings_rates: Vec::new(),
        savings_overrides: BTreeMap::new(),
        transactions: Vec::new(),
        incomes: (1..=4)
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
        savings_params: BTreeMap::new(),
    }
}

/// Долг 30 000 ₽ в январе на трату 30 000 ₽ и три погашения по 10 000 ₽ (февраль–апрель).
fn with_debt(paid: usize) -> DataSet {
    let mut data = base();
    data.transactions.push(Transaction {
        id: TxId(1),
        month: ym("2026-01"),
        category_id: CLOTHES,
        title: "Куртка".to_owned(),
        amount: rub(30_000),
        status: TxStatus::Debt,
        planned_amount: None,
    });
    data.debts.push(Debt {
        id: DEBT,
        lender: "Кредитная карта".to_owned(),
        amount: rub(30_000),
        taken_month: ym("2026-01"),
        category_id: Some(CLOTHES),
    });
    for (i, month) in ["2026-02", "2026-03", "2026-04"].into_iter().enumerate() {
        data.debt_payments.push(DebtPayment {
            debt_id: DEBT,
            month: ym(month),
            amount: rub(10_000),
            status: if i < paid {
                DebtPaymentStatus::Paid
            } else {
                DebtPaymentStatus::Planned
            },
        });
    }
    data
}

fn summary(data: &DataSet, month: &str) -> planning_budget_core::calc::MonthSummary {
    Ledger::new(data).unwrap().month_summary(ym(month)).unwrap()
}

#[test]
fn loan_for_an_expense_nets_to_zero_in_the_month_it_is_taken() {
    let data = with_debt(0);
    let january = summary(&data, "2026-01");
    assert_eq!(january.expenses, rub(30_000));
    assert_eq!(january.borrowed, rub(30_000));
    // Расход 30 000 ₽ и заём 30 000 ₽: свободный остаток дохода не уменьшился.
    assert_eq!(january.free, rub(100_000));
}

#[test]
fn repayment_lowers_free_but_is_not_an_expense() {
    let data = with_debt(3);
    for month in ["2026-02", "2026-03", "2026-04"] {
        let s = summary(&data, month);
        assert_eq!(s.expenses, Money::ZERO, "{month}: погашение не расход");
        assert_eq!(s.repaid, rub(10_000));
        assert_eq!(s.free, rub(90_000));
    }
}

#[test]
fn only_paid_repayments_reduce_free() {
    let data = with_debt(1);
    assert_eq!(summary(&data, "2026-02").free, rub(90_000));
    // Март и апрель ещё в плане: деньги не ушли.
    assert_eq!(summary(&data, "2026-03").free, rub(100_000));
    assert_eq!(summary(&data, "2026-04").repaid, Money::ZERO);
}

#[test]
fn without_debts_free_is_the_old_formula() {
    let mut data = base();
    data.transactions.push(Transaction {
        id: TxId(1),
        month: ym("2026-01"),
        category_id: CLOTHES,
        title: "Куртка".to_owned(),
        amount: rub(30_000),
        status: TxStatus::Paid,
        planned_amount: None,
    });
    let s = summary(&data, "2026-01");
    assert_eq!((s.borrowed, s.repaid), (Money::ZERO, Money::ZERO));
    assert_eq!(s.free, s.income.checked_sub(s.expenses).unwrap());
}

#[test]
fn cumulative_free_follows_the_loan_cycle() {
    let data = with_debt(3);
    // Четыре месяца дохода 400 000 ₽, расход 30 000 ₽, заём 30 000 ₽, погашено 30 000 ₽.
    assert_eq!(summary(&data, "2026-04").free_cum, rub(370_000));
}

#[test]
fn debt_state_tracks_remaining_next_payment_and_closing() {
    let ledger_state = |paid| {
        let data = with_debt(paid);
        Ledger::new(&data)
            .unwrap()
            .debt_state(DEBT)
            .unwrap()
            .unwrap()
    };
    let open = ledger_state(1);
    assert_eq!(open.remaining, rub(20_000));
    assert_eq!(open.next_payment.map(|p| p.month), Some(ym("2026-03")));
    assert!(!open.closed);

    let closed = ledger_state(3);
    assert_eq!(closed.remaining, Money::ZERO);
    assert_eq!(closed.next_payment, None);
    assert!(closed.closed);

    let data = base();
    assert!(
        Ledger::new(&data)
            .unwrap()
            .debt_state(DebtId(9))
            .unwrap()
            .is_none()
    );
}

#[test]
fn repayments_planned_sums_every_status_of_the_month() {
    let data = with_debt(1);
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(
        ledger.repayments_planned(ym("2026-02")).unwrap(),
        rub(10_000)
    );
    assert_eq!(
        ledger.repayments_planned(ym("2026-05")).unwrap(),
        Money::ZERO
    );
}

#[test]
fn equal_parts_put_the_kopeck_remainder_into_the_last_payment() {
    let rows = equal_parts_schedule(Money::from_kopecks(10_000), ym("2026-02"), 3).unwrap();
    let amounts: Vec<i64> = rows.iter().map(|(_, m)| m.kopecks()).collect();
    assert_eq!(amounts, vec![3_333, 3_333, 3_334]);
    let months: Vec<String> = rows.iter().map(|(m, _)| m.to_string()).collect();
    assert_eq!(months, vec!["2026-02", "2026-03", "2026-04"]);
}

#[test]
fn equal_parts_cross_the_year_boundary() {
    let rows = equal_parts_schedule(rub(300), ym("2026-11"), 3).unwrap();
    let months: Vec<String> = rows.iter().map(|(m, _)| m.to_string()).collect();
    assert_eq!(months, vec!["2026-11", "2026-12", "2027-01"]);
}

#[test]
fn schedules_reject_impossible_input() {
    let bad =
        |r: Result<Vec<(YearMonth, Money)>, CoreError>| matches!(r, Err(CoreError::Schedule(_)));
    assert!(bad(equal_parts_schedule(rub(100), ym("2026-01"), 0)));
    assert!(bad(equal_parts_schedule(rub(100), ym("2026-01"), 601)));
    assert!(bad(equal_parts_schedule(Money::ZERO, ym("2026-01"), 3)));
    assert!(bad(equal_parts_schedule(
        Money::from_kopecks(2),
        ym("2026-01"),
        3
    )));
    assert!(bad(single_payment_schedule(
        rub(100),
        ym("2026-03"),
        ym("2026-02")
    )));
    assert!(bad(single_payment_schedule(
        Money::ZERO,
        ym("2026-03"),
        ym("2026-03")
    )));
    assert_eq!(
        single_payment_schedule(rub(100), ym("2026-03"), ym("2026-05")).unwrap(),
        vec![(ym("2026-05"), rub(100))]
    );
}

#[test]
fn validate_schedule_checks_sum_months_and_duplicates() {
    let ok = vec![(ym("2026-02"), rub(60)), (ym("2026-03"), rub(40))];
    assert!(validate_schedule(rub(100), ym("2026-01"), &ok).is_ok());
    let cases: [(&str, Vec<(YearMonth, Money)>); 5] = [
        ("empty", vec![]),
        ("short", vec![(ym("2026-02"), rub(60))]),
        (
            "duplicate",
            vec![(ym("2026-02"), rub(50)), (ym("2026-02"), rub(50))],
        ),
        ("before the loan", vec![(ym("2025-12"), rub(100))]),
        (
            "non-positive",
            vec![(ym("2026-02"), rub(110)), (ym("2026-03"), rub(-10))],
        ),
    ];
    for (name, rows) in cases {
        assert!(
            matches!(
                validate_schedule(rub(100), ym("2026-01"), &rows),
                Err(CoreError::Schedule(_))
            ),
            "{name}"
        );
    }
}

proptest! {
    /// График «равными частями» всегда равен сумме долга: положительные платежи, подряд идущие месяцы.
    #[test]
    fn equal_parts_always_add_up_to_the_debt(
        kopecks in 1_i64..1_000_000_000_000,
        months in 1_u32..=120,
        start in 0_u32..48,
    ) {
        prop_assume!(kopecks >= i64::from(months));
        let first = (0..start).fold(ym("2024-01"), |m, _| m.succ().unwrap());
        let amount = Money::from_kopecks(kopecks);
        let rows = equal_parts_schedule(amount, first, months).unwrap();
        prop_assert_eq!(rows.len(), usize::try_from(months).unwrap());
        prop_assert!(validate_schedule(amount, first, &rows).is_ok());
        let sum: i64 = rows.iter().map(|(_, m)| m.kopecks()).sum();
        prop_assert_eq!(sum, kopecks);
        prop_assert!(rows.windows(2).all(|w| w[1].0 == w[0].0.succ().unwrap()));
    }
}

#[test]
fn debts_summary_reports_remaining_month_payments_and_counts() {
    let data = with_debt(1);
    let ledger = Ledger::new(&data).unwrap();
    let february = ledger.debts_summary(ym("2026-02")).unwrap();
    assert_eq!((february.open_count, february.closed_count), (1, 0));
    assert_eq!(february.remaining_total, rub(20_000));
    assert_eq!(february.payments_planned, rub(10_000));
    assert_eq!(february.payments_paid, rub(10_000));
    let march = ledger.debts_summary(ym("2026-03")).unwrap();
    assert_eq!(
        (march.payments_planned, march.payments_paid),
        (rub(10_000), Money::ZERO)
    );

    let closed = with_debt(3);
    let summary = Ledger::new(&closed)
        .unwrap()
        .debts_summary(ym("2026-04"))
        .unwrap();
    assert_eq!((summary.open_count, summary.closed_count), (0, 1));
    assert_eq!(summary.remaining_total, Money::ZERO);
}

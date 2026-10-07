//! Долги и графики погашений в хранилище: правила графика, закрытие, мягкое удаление,
//! учёт в наборе данных для расчётов.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use planning_budget_core::calc::Ledger;
use planning_budget_core::{
    CategoryId, DebtPaymentStatus, IncomeStatus, Money, TxStatus, YearMonth,
};
use planning_budget_storage::{
    Db, DebtPatch, NewDebt, NewIncome, NewTransaction, RecordSource, StorageError,
};
use tempfile::TempDir;

const KEY: [u8; 32] = [7; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn clothes(db: &Db) -> CategoryId {
    db.category_id_by_name("Одежда").unwrap().unwrap()
}

fn new_debt(db: &Db, schedule: Vec<(YearMonth, Money)>) -> NewDebt {
    NewDebt {
        lender: "Кредитная карта".to_owned(),
        amount: rub(30_000),
        taken_month: ym("2026-10"),
        taken_date: None,
        category_id: Some(clothes(db)),
        transaction_id: None,
        comment: None,
        schedule,
    }
}

fn three_parts() -> Vec<(YearMonth, Money)> {
    vec![
        (ym("2026-11"), rub(10_000)),
        (ym("2026-12"), rub(10_000)),
        (ym("2027-01"), rub(10_000)),
    ]
}

fn invalid(err: StorageError, key: &str) -> bool {
    matches!(err, StorageError::Invalid(k) if k == key)
}

#[test]
fn create_stores_debt_with_planned_schedule() {
    let (_dir, mut db) = seeded();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    assert_eq!(debt.amount, rub(30_000));
    assert_eq!(debt.payments.len(), 3);
    assert!(
        debt.payments
            .iter()
            .all(|p| p.status == DebtPaymentStatus::Planned)
    );
    assert!(!debt.closed);
    assert_eq!(db.debts(false).unwrap().len(), 1);
}

#[test]
fn schedule_must_add_up_and_start_no_earlier_than_the_loan() {
    let (_dir, mut db) = seeded();
    let short = vec![(ym("2026-11"), rub(10_000))];
    let err = db.debt_create(&new_debt(&db, short), now()).unwrap_err();
    assert!(invalid(err, "debt.schedule"));

    let early = vec![(ym("2026-09"), rub(30_000))];
    let err = db.debt_create(&new_debt(&db, early), now()).unwrap_err();
    assert!(invalid(err, "debt.schedule"));

    let duplicate = vec![(ym("2026-11"), rub(15_000)), (ym("2026-11"), rub(15_000))];
    let err = db
        .debt_create(&new_debt(&db, duplicate), now())
        .unwrap_err();
    assert!(invalid(err, "debt.schedule"));
    assert!(db.debts(true).unwrap().is_empty(), "ничего не записалось");
}

#[test]
fn lender_amount_and_date_are_validated() {
    let (_dir, mut db) = seeded();
    let mut empty = new_debt(&db, three_parts());
    empty.lender = "   ".to_owned();
    assert!(invalid(
        db.debt_create(&empty, now()).unwrap_err(),
        "debt.lender_empty"
    ));

    let mut zero = new_debt(&db, three_parts());
    zero.amount = Money::ZERO;
    assert!(db.debt_create(&zero, now()).is_err());

    let mut wrong_date = new_debt(&db, three_parts());
    wrong_date.taken_date = Some("2026-09-15".parse::<NaiveDate>().unwrap());
    assert!(invalid(
        db.debt_create(&wrong_date, now()).unwrap_err(),
        "debt.date_month_mismatch"
    ));
}

#[test]
fn paying_every_row_closes_the_debt_and_reverting_reopens_it() {
    let (_dir, mut db) = seeded();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    let ids: Vec<i64> = debt.payments.iter().map(|p| p.id).collect();
    for id in &ids {
        db.debt_payment_set_status(*id, DebtPaymentStatus::Paid, None, now())
            .unwrap();
    }
    assert!(db.debt(debt.id).unwrap().closed);
    assert!(
        db.debts(false).unwrap().is_empty(),
        "закрытые скрыты по умолчанию"
    );
    assert_eq!(db.debts(true).unwrap().len(), 1);

    let reopened = db
        .debt_payment_set_status(ids[2], DebtPaymentStatus::Planned, None, now())
        .unwrap();
    assert!(!reopened.closed);
}

#[test]
fn update_keeps_paid_rows_and_replaces_only_the_unpaid_ones() {
    let (_dir, mut db) = seeded();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    db.debt_payment_set_status(debt.payments[0].id, DebtPaymentStatus::Paid, None, now())
        .unwrap();

    // Оставшиеся 20 000 ₽ раскладываются на один платёж в феврале.
    let patch = DebtPatch {
        schedule: Some(vec![(ym("2027-02"), rub(20_000))]),
        ..DebtPatch::default()
    };
    let updated = db.debt_update(debt.id, &patch, now()).unwrap();
    let months: Vec<String> = updated
        .payments
        .iter()
        .map(|p| p.month.to_string())
        .collect();
    assert_eq!(months, vec!["2026-11", "2027-02"]);
    assert_eq!(updated.payments[0].status, DebtPaymentStatus::Paid);

    // График без оплаченной части не сходится с суммой долга.
    let bad = DebtPatch {
        schedule: Some(vec![(ym("2027-03"), rub(30_000))]),
        ..DebtPatch::default()
    };
    assert!(invalid(
        db.debt_update(debt.id, &bad, now()).unwrap_err(),
        "debt.schedule"
    ));
}

#[test]
fn changing_the_amount_requires_a_new_schedule() {
    let (_dir, mut db) = seeded();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    let only_amount = DebtPatch {
        amount: Some(rub(60_000)),
        ..DebtPatch::default()
    };
    assert!(invalid(
        db.debt_update(debt.id, &only_amount, now()).unwrap_err(),
        "debt.schedule"
    ));

    let with_schedule = DebtPatch {
        amount: Some(rub(60_000)),
        schedule: Some(vec![
            (ym("2026-11"), rub(30_000)),
            (ym("2026-12"), rub(30_000)),
        ]),
        ..DebtPatch::default()
    };
    let updated = db.debt_update(debt.id, &with_schedule, now()).unwrap();
    assert_eq!(updated.amount, rub(60_000));
    assert_eq!(updated.payments.len(), 2);
}

#[test]
fn metadata_edit_does_not_touch_the_schedule() {
    let (_dir, mut db) = seeded();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    let patch = DebtPatch {
        lender: Some("Брат".to_owned()),
        comment: Some(Some("до зарплаты".to_owned())),
        category_id: Some(None),
        ..DebtPatch::default()
    };
    let updated = db.debt_update(debt.id, &patch, now()).unwrap();
    assert_eq!(updated.lender, "Брат");
    assert_eq!(updated.comment.as_deref(), Some("до зарплаты"));
    assert_eq!(updated.category_id, None);
    assert_eq!(updated.payments, debt.payments);
}

#[test]
fn delete_hides_the_debt_and_restore_brings_it_back() {
    let (_dir, mut db) = seeded();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    db.debt_delete(debt.id, now()).unwrap();
    assert!(matches!(db.debt(debt.id), Err(StorageError::NotFound)));
    assert!(
        db.dataset(ym("2026-10"), ym("2026-12"))
            .unwrap()
            .debts
            .is_empty()
    );
    let restored = db.debt_restore(debt.id, now()).unwrap();
    assert_eq!(restored.payments.len(), 3);
}

#[test]
fn debt_from_a_transaction_takes_amount_category_and_month_from_it() {
    let (_dir, mut db) = seeded();
    let category = clothes(&db);
    let tx = db
        .transaction_create(
            &NewTransaction {
                month: ym("2026-10"),
                date: Some("2026-10-05".parse().unwrap()),
                category_id: category,
                title: "Куртка".to_owned(),
                amount: rub(30_000),
                status: TxStatus::Debt,
                comment: None,
                source: RecordSource::Manual,
            },
            now(),
        )
        .unwrap();
    let debt = db
        .debt_create_from_transaction(tx.id, "Кредитная карта", three_parts(), now())
        .unwrap();
    assert_eq!(debt.amount, rub(30_000));
    assert_eq!(debt.taken_month, ym("2026-10"));
    assert_eq!(debt.category_id, Some(category));
    assert_eq!(debt.transaction_id, Some(tx.id));
    assert_eq!(debt.taken_date, Some("2026-10-05".parse().unwrap()));
}

#[test]
fn dataset_feeds_borrowed_and_repaid_into_free() {
    let (_dir, mut db) = seeded();
    db.income_create(
        &NewIncome {
            month: ym("2026-10"),
            date: None,
            source_name: "Зарплата".to_owned(),
            amount: rub(100_000),
            status: IncomeStatus::Received,
            comment: None,
            source: RecordSource::Manual,
        },
        now(),
    )
    .unwrap();
    let debt = db
        .debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    db.debt_payment_set_status(debt.payments[0].id, DebtPaymentStatus::Paid, None, now())
        .unwrap();

    let data = db.dataset(ym("2026-10"), ym("2026-12")).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    let october = ledger.month_summary(ym("2026-10")).unwrap();
    assert_eq!(october.borrowed, rub(30_000));
    assert_eq!(october.free, rub(130_000), "заём — источник денег месяца");
    let november = ledger.month_summary(ym("2026-11")).unwrap();
    assert_eq!(november.repaid, rub(10_000));
    assert_eq!(november.expenses, Money::ZERO);
}

#[test]
fn second_debt_from_the_same_transaction_is_a_conflict() {
    let (_dir, mut db) = seeded();
    let tx = db
        .transaction_create(
            &NewTransaction {
                month: ym("2026-10"),
                date: None,
                category_id: clothes(&db),
                title: "Куртка".to_owned(),
                amount: rub(30_000),
                status: TxStatus::Debt,
                comment: None,
                source: RecordSource::Manual,
            },
            now(),
        )
        .unwrap();
    db.debt_create_from_transaction(tx.id, "Карта", three_parts(), now())
        .unwrap();
    let err = db
        .debt_create_from_transaction(tx.id, "Карта", three_parts(), now())
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::Conflict("debt.transaction_linked")
    ));
}

#[test]
fn category_with_a_debt_cannot_be_deleted() {
    let (_dir, mut db) = seeded();
    db.debt_create(&new_debt(&db, three_parts()), now())
        .unwrap();
    let err = db.category_delete(clothes(&db)).unwrap_err();
    assert!(matches!(err, StorageError::Conflict("category.in_use")));
}

#[test]
fn moving_a_transaction_to_another_month_drops_its_planned_snapshot() {
    use planning_budget_storage::TransactionPatch;
    let (_dir, mut db) = seeded();
    let tx = db
        .transaction_create(
            &NewTransaction {
                month: ym("2026-10"),
                date: None,
                category_id: clothes(&db),
                title: "Куртка".to_owned(),
                amount: rub(5_000),
                status: TxStatus::Paid,
                comment: None,
                source: RecordSource::Manual,
            },
            now(),
        )
        .unwrap();
    db.conn()
        .execute(
            "UPDATE transactions SET planned_amount = 500000 WHERE id = ?1",
            [tx.id.0],
        )
        .unwrap();
    let planned = |db: &Db| -> Option<i64> {
        db.conn()
            .query_row(
                "SELECT planned_amount FROM transactions WHERE id = ?1",
                [tx.id.0],
                |r| r.get(0),
            )
            .unwrap()
    };
    let same_month = TransactionPatch {
        title: Some("Куртка зимняя".to_owned()),
        ..TransactionPatch::default()
    };
    db.transaction_update(tx.id, &same_month, now()).unwrap();
    assert_eq!(planned(&db), Some(500_000));
    let moved = TransactionPatch {
        month: Some(ym("2026-11")),
        ..TransactionPatch::default()
    };
    db.transaction_update(tx.id, &moved, now()).unwrap();
    assert_eq!(planned(&db), None);
}

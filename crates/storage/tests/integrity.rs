//! Целостность: границы сумм, атомарность настроек, архив сбережений с будущими процентами.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::{BasisPoints, CategoryKind, Money, TxStatus, YearMonth};
use planning_budget_storage::{Db, NewCategory, NewTransaction, RecordSource, StorageError};
use serde_json::json;
use tempfile::TempDir;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &[5; 32]).unwrap();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn tx_with(db: &Db, amount: i64) -> NewTransaction {
    NewTransaction {
        month: ym("2026-10"),
        date: None,
        category_id: db.category_id_by_name("Продукты").unwrap().unwrap(),
        title: "x".to_owned(),
        amount: Money::from_kopecks(amount),
        status: TxStatus::Paid,
        comment: None,
        source: RecordSource::Manual,
    }
}

#[test]
fn amount_has_an_upper_bound() {
    let (_dir, mut db) = seeded();
    let max = 100_000_000_000_000;
    db.transaction_create(&tx_with(&db, max), now()).unwrap();
    assert!(matches!(
        db.transaction_create(&tx_with(&db, max + 1), now()),
        Err(StorageError::Invalid("record.amount_too_large"))
    ));
    assert!(matches!(
        db.transaction_create(&tx_with(&db, i64::MAX), now()),
        Err(StorageError::Invalid("record.amount_too_large"))
    ));
}

#[test]
fn settings_batch_is_atomic() {
    let (_dir, mut db) = seeded();
    let err = db
        .settings_set_many(&[
            ("savings.target_norm_bp", json!(1450)),
            ("ui.weeks_per_month", json!(9)),
        ])
        .unwrap_err();
    assert!(matches!(err, StorageError::Invalid("settings.bad_value")));
    assert_eq!(db.settings().unwrap().savings_norm.0, 1400);
}

#[test]
fn corridor_bounds_must_stay_ordered_and_failure_rolls_back() {
    let (_dir, mut db) = seeded();
    let err = db
        .settings_set_many(&[
            ("savings.target_min_bp", json!(1600)),
            ("ui.weeks_per_month", json!(3)),
        ])
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::Invalid("settings.corridor_order")
    ));
    let s = db.settings().unwrap();
    assert_eq!((s.savings_min.0, s.weeks_per_month), (1300, 4));

    db.settings_set_many(&[
        ("savings.target_min_bp", json!(1000)),
        ("savings.target_norm_bp", json!(1100)),
        ("savings.target_max_bp", json!(1200)),
    ])
    .unwrap();
    assert_eq!(db.settings().unwrap().savings_max.0, 1200);
}

#[test]
fn archiving_a_savings_category_drops_its_future_rates_and_overrides() {
    let (_dir, mut db) = seeded();
    let reserve = db
        .category_create(
            &NewCategory {
                name: "Подушка".to_owned(),
                kind: CategoryKind::Savings,
                color: "#5B7FD6".to_owned(),
                note: None,
            },
            now(),
        )
        .unwrap();
    db.savings_rate_set(reserve.id, ym("2026-10"), BasisPoints(500))
        .unwrap();
    db.savings_rate_set(reserve.id, ym("2026-12"), BasisPoints(700))
        .unwrap();
    db.savings_override_set(ym("2026-11"), reserve.id, BasisPoints(900))
        .unwrap();

    db.category_archive(reserve.id, ym("2026-10"), now())
        .unwrap();
    let rates: Vec<_> = db
        .savings_rates()
        .unwrap()
        .into_iter()
        .filter(|r| r.category_id == reserve.id)
        .map(|r| (r.valid_from, r.rate))
        .collect();
    assert_eq!(rates, vec![(ym("2026-10"), BasisPoints(0))]);
    assert!(db.savings_overrides().unwrap().is_empty());
}

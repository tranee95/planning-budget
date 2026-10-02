//! `dataset(from, to)`, настройки и заливка сида: данные из базы совпадают с эталонным `DataSet`.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use budget_core::calc::Ledger;
use budget_core::{DataSet, Money, TxStatus, YearMonth, load_seed};
use budget_storage::{Db, NewTransaction, RecordSource, StorageError};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use tempfile::TempDir;

const KEY: [u8; 32] = [5; 32];
const SEED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/seed-2026.json"
));

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn empty() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn with_seed() -> (TempDir, Db, DataSet) {
    let (dir, mut db) = empty();
    let data = load_seed(SEED).unwrap();
    db.load_dataset(&data, now()).unwrap();
    (dir, db, data)
}

fn sorted(mut data: DataSet) -> DataSet {
    data.transactions.sort_by_key(|t| t.id);
    data.incomes.sort_by_key(|i| i.id);
    data
}

#[test]
fn seed_roundtrips_through_the_database_unchanged() {
    let (_dir, db, original) = with_seed();
    let loaded = db.dataset(ym("2026-01"), ym("2026-12")).unwrap();
    assert_eq!(sorted(loaded), sorted(original));
}

#[test]
fn ledger_over_database_data_gives_the_golden_year_totals() {
    let (_dir, db, _) = with_seed();
    let data = db.dataset(ym("2026-01"), ym("2026-12")).unwrap();
    let year = Ledger::new(&data)
        .unwrap()
        .year_summary(2026, ym("2026-09"))
        .unwrap();
    assert_eq!(year.months_with_data, 9);
    assert_eq!(year.income, Money::from_kopecks(1_330_000 * 100));
}

#[test]
fn dataset_is_limited_to_the_period_and_skips_deleted_rows() {
    let (_dir, mut db, _) = with_seed();
    let march = db.dataset(ym("2026-03"), ym("2026-03")).unwrap();
    assert!(!march.transactions.is_empty());
    assert!(march.transactions.iter().all(|t| t.month == ym("2026-03")));
    assert!(march.incomes.iter().all(|i| i.month == ym("2026-03")));
    assert_eq!(march.categories.len(), 17);

    let victim = march.transactions[0].id;
    db.transaction_delete(victim, now()).unwrap();
    let after = db.dataset(ym("2026-03"), ym("2026-03")).unwrap();
    assert_eq!(after.transactions.len(), march.transactions.len() - 1);
    assert!(after.transactions.iter().all(|t| t.id != victim));
}

#[test]
fn new_records_show_up_in_the_dataset() {
    let (_dir, mut db, _) = with_seed();
    let category = db.category_id_by_name("Продукты").unwrap().unwrap();
    let created = db
        .transaction_create(
            &NewTransaction {
                month: ym("2026-10"),
                date: None,
                category_id: category,
                title: "Лента".to_owned(),
                amount: Money::from_kopecks(12_300),
                status: TxStatus::Planned,
                comment: None,
                source: RecordSource::Manual,
            },
            now(),
        )
        .unwrap();
    let data = db.dataset(ym("2026-10"), ym("2026-10")).unwrap();
    assert_eq!(data.transactions.len(), 1);
    assert_eq!(data.transactions[0].id, created.id);
    assert_eq!(data.transactions[0].status, TxStatus::Planned);
}

#[test]
fn dev_seed_refuses_to_overwrite_existing_records() {
    let (_dir, mut db, data) = with_seed();
    assert!(matches!(
        db.load_dataset(&data, now()),
        Err(StorageError::Conflict("seed.not_empty"))
    ));
}

#[test]
fn settings_are_typed_and_validated() {
    let (_dir, mut db) = empty();
    let s = db.settings().unwrap();
    assert_eq!(s.savings_norm.0, 1400);
    assert_eq!(s.weeks_per_month, 4);
    // Месяц создания хранилища (сид выполнен 2026-10-01).
    assert_eq!(s.bonds_initial_month, ym("2026-10"));

    db.setting_set("savings.target_norm_bp", &json!(1500))
        .unwrap();
    db.setting_set("ui.weeks_per_month", &json!(5)).unwrap();
    db.setting_set("bonds.initial_month", &json!("2025-12"))
        .unwrap();
    let s = db.settings().unwrap();
    assert_eq!(
        (s.savings_norm.0, s.weeks_per_month, s.bonds_initial_month),
        (1500, 5, ym("2025-12"))
    );

    for (key, value) in [
        ("savings.target_norm_bp", json!(10_001)),
        ("savings.target_norm_bp", json!("1500")),
        ("ui.weeks_per_month", json!(0)),
        ("bonds.initial_month", json!("2026-13")),
        ("security.lock_on_minimize", json!(1)),
        ("currency", json!(" ")),
    ] {
        assert!(
            matches!(
                db.setting_set(key, &value),
                Err(StorageError::Invalid("settings.bad_value"))
            ),
            "{key} = {value}"
        );
    }
    assert!(matches!(
        db.setting_set("unknown.key", &json!(1)),
        Err(StorageError::Invalid("settings.unknown_key"))
    ));
}

#[test]
fn dataset_uses_a_fixed_number_of_statements_per_period() {
    let (_dir, db, _) = with_seed();
    let plan: Vec<String> = {
        let mut stmt = db
            .conn()
            .prepare(
                "EXPLAIN QUERY PLAN SELECT id, month, category_id, title, amount, status
                 FROM v_transactions WHERE month BETWEEN ?1 AND ?2
                 ORDER BY month, category_id, sort_key, id",
            )
            .unwrap();
        stmt.query_map(["2026-01", "2026-12"], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert!(
        plan.iter()
            .all(|step| !step.starts_with("SCAN transactions")),
        "{plan:?}"
    );
}

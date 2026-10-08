//! Запись разобранной старой таблицы в базу.
//!
//! Книга собирается в коде: файла таблицы в репозитории нет.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::{
    BasisPoints, CategoryKind, IncomeStatus, LegacyBook, LegacyCategory, LegacyIncome,
    LegacySettings, LegacyTransaction, Money, TxStatus, YearMonth,
};
use planning_budget_storage::{Db, StorageError};
use tempfile::TempDir;

const KEY: [u8; 32] = [8; 32];

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

fn count(db: &Db, sql: &str) -> i64 {
    db.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Существующая категория базы в верхнем регистре и новая категория «Хобби».
fn book(db: &Db) -> LegacyBook {
    let existing = db.categories(false).unwrap();
    let first = existing
        .iter()
        .find(|c| c.kind != CategoryKind::Savings)
        .unwrap();
    LegacyBook {
        categories: vec![
            LegacyCategory {
                name: first.name.to_uppercase(),
                kind: first.kind,
                limit: Some(rub(5_000)),
                note: Some("из таблицы".to_owned()),
            },
            LegacyCategory {
                name: "Хобби".to_owned(),
                kind: CategoryKind::Wants,
                limit: Some(rub(1_500)),
                note: None,
            },
        ],
        settings: None,
        transactions: vec![
            LegacyTransaction {
                month: ym("2026-02"),
                category: "Хобби".to_owned(),
                title: "краски".to_owned(),
                amount: rub(700),
                status: TxStatus::Paid,
            },
            LegacyTransaction {
                month: ym("2026-03"),
                category: first.name.to_uppercase(),
                title: "обед".to_owned(),
                amount: rub(300),
                status: TxStatus::Planned,
            },
        ],
        incomes: vec![LegacyIncome {
            month: ym("2026-01"),
            source_name: "Зарплата".to_owned(),
            amount: rub(100_000),
            status: IncomeStatus::Received,
        }],
        block_mismatches: Vec::new(),
    }
}

#[test]
fn import_merges_by_name_creates_new_categories_and_writes_records() {
    let (_dir, mut db) = seeded();
    let before = db.categories(false).unwrap().len();
    let book = book(&db);

    let report = db.import_legacy(&book, now()).unwrap();

    assert_eq!(report.categories_matched, 1);
    assert_eq!(report.categories_created, 1);
    assert!(report.kind_conflicts.is_empty());
    assert_eq!(report.limits_written, 2);
    assert_eq!((report.transactions, report.incomes), (2, 1));
    assert!(!report.settings_applied);
    assert_eq!(db.categories(false).unwrap().len(), before + 1);
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM transactions WHERE source = 'legacy'"
        ),
        2
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM incomes WHERE source = 'legacy'"),
        1
    );
    // Лимиты действуют с первого месяца данных (в книге это январь).
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM category_limits WHERE valid_from = '2026-01' AND amount IN (500000, 150000)"
        ),
        2
    );
    // Найденная категория получила заметку из файла.
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM categories WHERE note = 'из таблицы'"
        ),
        1
    );
}

#[test]
fn kind_mismatch_is_reported_and_the_kind_is_kept() {
    let (_dir, mut db) = seeded();
    let mut book = book(&db);
    let seeded_kind = book.categories[0].kind;
    book.categories[0].kind = if seeded_kind == CategoryKind::Wants {
        CategoryKind::Mandatory
    } else {
        CategoryKind::Wants
    };
    let name = book.categories[0].name.clone();

    let report = db.import_legacy(&book, now()).unwrap();

    assert_eq!(report.kind_conflicts, [name]);
    assert!(
        db.categories(false)
            .unwrap()
            .iter()
            .any(|c| c.kind == seeded_kind)
    );
}

#[test]
fn settings_set_targets_and_savings_params() {
    let (_dir, mut db) = seeded();
    let mut book = book(&db);
    book.settings = Some(LegacySettings {
        savings_min: BasisPoints(1000),
        savings_norm: BasisPoints(1200),
        savings_max: BasisPoints(1800),
        bonds_rate: BasisPoints(1700),
        bonds_coupon_tax: BasisPoints(1300),
        bonds_initial_balance: rub(250_000),
    });

    let report = db.import_legacy(&book, now()).unwrap();

    assert!(report.settings_applied);
    let value = |key: &str| -> String {
        db.conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .unwrap()
    };
    assert_eq!(value("savings.target_min_bp"), "1000");
    assert_eq!(value("savings.target_norm_bp"), "1200");
    assert_eq!(value("savings.target_max_bp"), "1800");
    assert_eq!(
        count(
            &db,
            "SELECT count(*) FROM savings_params WHERE annual_rate_bp = 1700 AND initial_balance = 25000000"
        ),
        1
    );
}

#[test]
fn second_import_is_refused_and_leaves_data_untouched() {
    let (_dir, mut db) = seeded();
    let book = book(&db);
    db.import_legacy(&book, now()).unwrap();

    let err = db.import_legacy(&book, now()).unwrap_err();

    assert!(matches!(err, StorageError::Conflict("legacy.not_empty")));
    assert_eq!(count(&db, "SELECT count(*) FROM transactions"), 2);
}

#[test]
fn empty_book_is_invalid() {
    let (_dir, mut db) = seeded();
    let err = db.import_legacy(&LegacyBook::default(), now()).unwrap_err();
    assert!(matches!(err, StorageError::Invalid("legacy.empty")));
}

#[test]
fn unknown_category_rolls_everything_back() {
    let (_dir, mut db) = seeded();
    let mut book = book(&db);
    book.transactions.push(LegacyTransaction {
        month: ym("2026-04"),
        category: "Нет такой".to_owned(),
        title: "?".to_owned(),
        amount: rub(1),
        status: TxStatus::Paid,
    });
    let categories_before = count(&db, "SELECT count(*) FROM categories");

    let err = db.import_legacy(&book, now()).unwrap_err();

    assert!(matches!(
        err,
        StorageError::Invalid("legacy.unknown_category")
    ));
    assert_eq!(
        count(&db, "SELECT count(*) FROM categories"),
        categories_before
    );
    assert_eq!(count(&db, "SELECT count(*) FROM transactions"), 0);
    assert_eq!(count(&db, "SELECT count(*) FROM incomes"), 0);
}

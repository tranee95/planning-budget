//! Репозитории трат, доходов и тегов: правила месяца и даты, мягкое удаление, индекс поиска.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use budget_core::{CategoryId, IncomeStatus, Money, TxStatus, YearMonth};
use budget_storage::{
    Db, IncomePatch, NewIncome, NewTransaction, RecordSource, StorageError, TransactionPatch,
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use tempfile::TempDir;

const KEY: [u8; 32] = [5; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn day(s: &str) -> NaiveDate {
    s.parse().unwrap()
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

fn food(db: &Db) -> CategoryId {
    db.category_id_by_name("Продукты").unwrap().unwrap()
}

fn new_tx(category: CategoryId, month: &str, title: &str) -> NewTransaction {
    NewTransaction {
        month: ym(month),
        date: None,
        category_id: category,
        title: title.to_owned(),
        amount: rub(100),
        status: TxStatus::Paid,
        comment: None,
        source: RecordSource::Manual,
    }
}

fn new_income(month: &str, name: &str) -> NewIncome {
    NewIncome {
        month: ym(month),
        date: None,
        source_name: name.to_owned(),
        amount: rub(50_000),
        status: IncomeStatus::Received,
        comment: None,
        source: RecordSource::Manual,
    }
}

fn found(db: &Db, query: &str, kind: &str) -> Vec<i64> {
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT ref_id FROM search_fts WHERE search_fts MATCH ?1 AND kind = ?2 ORDER BY ref_id",
        )
        .unwrap();
    stmt.query_map([query, kind], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn created_transactions_keep_order_within_their_block() {
    let (_dir, mut db) = seeded();
    let cat = food(&db);
    let a = db
        .transaction_create(&new_tx(cat, "2026-10", "Лента"), now())
        .unwrap();
    let b = db
        .transaction_create(&new_tx(cat, "2026-10", "Пятёрочка"), now())
        .unwrap();
    db.transaction_create(&new_tx(cat, "2026-09", "Прошлый месяц"), now())
        .unwrap();
    assert_eq!((a.sort_key, b.sort_key), (0, 1));
    let titles: Vec<_> = db
        .transactions_of_month(ym("2026-10"))
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, ["Лента", "Пятёрочка"]);
}

#[test]
fn create_validates_amount_title_category_and_date() {
    let (_dir, mut db) = seeded();
    let cat = food(&db);
    let mut zero = new_tx(cat, "2026-10", "x");
    zero.amount = Money::ZERO;
    assert!(matches!(
        db.transaction_create(&zero, now()),
        Err(StorageError::Invalid("record.amount_not_positive"))
    ));
    assert!(matches!(
        db.transaction_create(&new_tx(cat, "2026-10", "  "), now()),
        Err(StorageError::Invalid("record.title_empty"))
    ));
    assert!(matches!(
        db.transaction_create(&new_tx(CategoryId(9999), "2026-10", "x"), now()),
        Err(StorageError::NotFound)
    ));
    let mut mismatch = new_tx(cat, "2026-10", "x");
    mismatch.date = Some(day("2026-09-30"));
    assert!(matches!(
        db.transaction_create(&mismatch, now()),
        Err(StorageError::Invalid("record.date_month_mismatch"))
    ));
    db.category_archive(cat, ym("2026-10"), now()).unwrap();
    assert!(matches!(
        db.transaction_create(&new_tx(cat, "2026-10", "x"), now()),
        Err(StorageError::Invalid("record.category_archived"))
    ));
}

#[test]
fn update_returns_previous_and_current_and_moves_between_blocks() {
    let (_dir, mut db) = seeded();
    let cat = food(&db);
    let other = db.category_id_by_name("Транспорт").unwrap().unwrap();
    let created = db
        .transaction_create(&new_tx(cat, "2026-10", "Лента"), now())
        .unwrap();
    let patch = TransactionPatch {
        amount: Some(rub(250)),
        status: Some(TxStatus::Debt),
        comment: Some(Some("чек".to_owned())),
        category_id: Some(other),
        ..TransactionPatch::default()
    };
    let result = db.transaction_update(created.id, &patch, now()).unwrap();
    assert_eq!(result.previous, created);
    assert_eq!(result.current.amount, rub(250));
    assert_eq!(result.current.status, TxStatus::Debt);
    assert_eq!(result.current.comment.as_deref(), Some("чек"));
    assert_eq!(result.current.category_id, other);
}

#[test]
fn month_and_date_move_together() {
    let (_dir, mut db) = seeded();
    let cat = food(&db);
    let mut input = new_tx(cat, "2026-10", "Лента");
    input.date = Some(day("2026-10-05"));
    let tx = db.transaction_create(&input, now()).unwrap();

    let new_date = TransactionPatch {
        date: Some(Some(day("2026-11-02"))),
        ..TransactionPatch::default()
    };
    let moved = db
        .transaction_update(tx.id, &new_date, now())
        .unwrap()
        .current;
    assert_eq!(
        (moved.month, moved.date),
        (ym("2026-11"), Some(day("2026-11-02")))
    );

    let new_month = TransactionPatch {
        month: Some(ym("2026-12")),
        ..TransactionPatch::default()
    };
    let moved = db
        .transaction_update(tx.id, &new_month, now())
        .unwrap()
        .current;
    assert_eq!((moved.month, moved.date), (ym("2026-12"), None));

    let conflicting = TransactionPatch {
        month: Some(ym("2026-12")),
        date: Some(Some(day("2026-01-01"))),
        ..TransactionPatch::default()
    };
    assert!(matches!(
        db.transaction_update(tx.id, &conflicting, now()),
        Err(StorageError::Invalid("record.date_month_mismatch"))
    ));
}

#[test]
fn soft_delete_hides_from_lists_and_search_and_restore_brings_back() {
    let (_dir, mut db) = seeded();
    let tx = db
        .transaction_create(&new_tx(food(&db), "2026-10", "Магнит"), now())
        .unwrap();
    assert_eq!(found(&db, "магнит*", "tx"), [tx.id.0]);

    db.transaction_delete(tx.id, now()).unwrap();
    assert!(db.transactions_of_month(ym("2026-10")).unwrap().is_empty());
    assert!(found(&db, "магнит*", "tx").is_empty());
    assert!(matches!(
        db.transaction_delete(tx.id, now()),
        Err(StorageError::NotFound)
    ));
    let patch = TransactionPatch {
        amount: Some(rub(1)),
        ..TransactionPatch::default()
    };
    assert!(matches!(
        db.transaction_update(tx.id, &patch, now()),
        Err(StorageError::NotFound)
    ));

    let restored = db.transaction_restore(tx.id, now()).unwrap();
    assert_eq!(restored.title, "Магнит");
    assert_eq!(found(&db, "магнит*", "tx"), [tx.id.0]);
    assert!(matches!(
        db.transaction_restore(tx.id, now()),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn restore_conflicts_when_the_fingerprint_was_taken_meanwhile() {
    let (_dir, mut db) = seeded();
    let cat = food(&db);
    let first = db
        .transaction_create(&new_tx(cat, "2026-10", "Лента"), now())
        .unwrap();
    let second = db
        .transaction_create(&new_tx(cat, "2026-10", "Лента"), now())
        .unwrap();
    db.conn()
        .execute(
            "UPDATE transactions SET fingerprint = 'fp' WHERE id = ?1",
            [first.id.0],
        )
        .unwrap();
    db.transaction_delete(first.id, now()).unwrap();
    db.conn()
        .execute(
            "UPDATE transactions SET fingerprint = 'fp' WHERE id = ?1",
            [second.id.0],
        )
        .unwrap();
    assert!(matches!(
        db.transaction_restore(first.id, now()),
        Err(StorageError::Conflict("record.duplicate"))
    ));
}

#[test]
fn tags_attach_to_transactions_and_rename_reindexes() {
    let (_dir, mut db) = seeded();
    let tx = db
        .transaction_create(&new_tx(food(&db), "2026-10", "Лента"), now())
        .unwrap();
    let trip = db.tag_create("Поездка").unwrap();
    let work = db.tag_create("Работа").unwrap();
    assert!(matches!(
        db.tag_create("поездка"),
        Err(StorageError::Conflict("tag.name_taken"))
    ));
    db.transaction_tags_set(tx.id, &[trip.id, work.id]).unwrap();
    assert_eq!(db.transaction(tx.id).unwrap().tags, [trip.id, work.id]);
    assert_eq!(
        db.transactions_of_month(ym("2026-10")).unwrap()[0]
            .tags
            .len(),
        2
    );
    assert_eq!(found(&db, "поездка*", "tx"), [tx.id.0]);

    db.tag_rename(trip.id, "Отпуск").unwrap();
    assert!(found(&db, "поездка*", "tx").is_empty());
    assert_eq!(found(&db, "отпуск*", "tx"), [tx.id.0]);

    db.tag_delete(trip.id).unwrap();
    assert_eq!(db.transaction(tx.id).unwrap().tags, [work.id]);
    assert!(matches!(
        db.transaction_tags_set(tx.id, &[trip.id]),
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        db.tag_delete(trip.id),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn incomes_follow_the_same_rules() {
    let (_dir, mut db) = seeded();
    let salary = db
        .income_create(&new_income("2026-10", "Зарплата"), now())
        .unwrap();
    let mut bad = new_income("2026-10", "x");
    bad.amount = rub(-5);
    assert!(matches!(
        db.income_create(&bad, now()),
        Err(StorageError::Invalid("record.amount_not_positive"))
    ));
    let patch = IncomePatch {
        status: Some(IncomeStatus::Expected),
        amount: Some(rub(60_000)),
        ..IncomePatch::default()
    };
    let result = db.income_update(salary.id, &patch, now()).unwrap();
    assert_eq!(result.previous.status, IncomeStatus::Received);
    assert_eq!(result.current.status, IncomeStatus::Expected);

    assert_eq!(found(&db, "зарплата*", "inc").len(), 1);
    db.income_delete(salary.id, now()).unwrap();
    assert!(db.incomes_of_month(ym("2026-10")).unwrap().is_empty());
    assert!(found(&db, "зарплата*", "inc").is_empty());
    assert_eq!(
        db.income_restore(salary.id, now()).unwrap().amount,
        rub(60_000)
    );
    assert!(matches!(
        db.income_delete(budget_core::IncomeId(9999), now()),
        Err(StorageError::NotFound)
    ));
}

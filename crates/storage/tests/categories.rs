//! Репозитории категорий, лимитов и плана сбережений.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::{BasisPoints, CategoryId, CategoryKind, Money, YearMonth};
use planning_budget_storage::{CategoryPatch, Db, NewCategory, StorageError};
use tempfile::TempDir;

const KEY: [u8; 32] = [5; 32];

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

fn new(name: &str, kind: CategoryKind) -> NewCategory {
    NewCategory {
        name: name.to_owned(),
        kind,
        color: "#3AA567".to_owned(),
        note: None,
    }
}

fn id_of(db: &Db, name: &str) -> CategoryId {
    db.category_id_by_name(name).unwrap().unwrap()
}

fn insert_tx(db: &Db, category: CategoryId, month: &str) {
    db.conn()
        .execute(
            "INSERT INTO transactions (month, category_id, title, amount, status, created_at, updated_at)
             VALUES (?1, ?2, 'тест', 10000, 'paid', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z')",
            rusqlite::params![month, category.0],
        )
        .unwrap();
}

#[test]
fn create_appends_to_the_end_and_rejects_taken_name_ignoring_case_and_yo() {
    let (_dir, mut db) = seeded();
    let last = db.categories(false).unwrap().last().unwrap().sort_order;
    let c = db
        .category_create(&new("  Ёлки ", CategoryKind::Wants), now())
        .unwrap();
    assert_eq!((c.name.as_str(), c.sort_order), ("Ёлки", last + 1));
    let err = db
        .category_create(&new("елки", CategoryKind::Wants), now())
        .unwrap_err();
    assert!(matches!(err, StorageError::Conflict("category.name_taken")));
}

#[test]
fn create_validates_name_and_color() {
    let (_dir, mut db) = seeded();
    let empty = db.category_create(&new("   ", CategoryKind::Wants), now());
    assert!(matches!(
        empty,
        Err(StorageError::Invalid("category.name_empty"))
    ));
    let mut bad = new("Кино", CategoryKind::Wants);
    bad.color = "red".to_owned();
    assert!(matches!(
        db.category_create(&bad, now()),
        Err(StorageError::Invalid("category.color_invalid"))
    ));
}

#[test]
fn update_changes_fields_and_keeps_the_rest() {
    let (_dir, mut db) = seeded();
    let id = id_of(&db, "Продукты");
    let before = db.category(id).unwrap();
    let patch = CategoryPatch {
        color: Some("#5B7FD6".to_owned()),
        note: Some(Some("еда".to_owned())),
        ..CategoryPatch::default()
    };
    let after = db.category_update(id, &patch, now()).unwrap();
    assert_eq!(after.name, before.name);
    assert_eq!(after.color, "#5B7FD6");
    assert_eq!(after.note.as_deref(), Some("еда"));
    let cleared = CategoryPatch {
        note: Some(None),
        ..CategoryPatch::default()
    };
    assert_eq!(db.category_update(id, &cleared, now()).unwrap().note, None);
}

#[test]
fn rename_to_another_active_name_conflicts() {
    let (_dir, mut db) = seeded();
    let id = id_of(&db, "Продукты");
    let patch = CategoryPatch {
        name: Some("сбережения".to_owned()),
        ..CategoryPatch::default()
    };
    assert!(matches!(
        db.category_update(id, &patch, now()),
        Err(StorageError::Conflict("category.name_taken"))
    ));
}

#[test]
fn reorder_requires_the_exact_set_and_applies_positions() {
    let (_dir, mut db) = seeded();
    let mut ids: Vec<CategoryId> = db.categories(false).unwrap().iter().map(|c| c.id).collect();
    ids.reverse();
    db.categories_reorder(&ids, now()).unwrap();
    let after: Vec<CategoryId> = db.categories(false).unwrap().iter().map(|c| c.id).collect();
    assert_eq!(after, ids);
    ids.pop();
    assert!(matches!(
        db.categories_reorder(&ids, now()),
        Err(StorageError::Invalid("category.reorder_mismatch"))
    ));
}

#[test]
fn archive_hides_category_and_frees_its_name() {
    let (_dir, mut db) = seeded();
    let id = id_of(&db, "Продукты");
    db.category_archive(id, ym("2026-10"), now()).unwrap();
    assert!(db.categories(false).unwrap().iter().all(|c| c.id != id));
    assert!(db.category(id).unwrap().archived);
    db.category_create(&new("Продукты", CategoryKind::Mandatory), now())
        .unwrap();
    assert!(matches!(
        db.category_unarchive(id, now()),
        Err(StorageError::Conflict("category.name_taken"))
    ));
}

#[test]
fn archiving_a_savings_category_zeroes_its_rate_from_that_month() {
    let (_dir, mut db) = seeded();
    let extra = db
        .category_create(&new("Подушка", CategoryKind::Savings), now())
        .unwrap();
    db.savings_rate_set(extra.id, ym("2026-10"), BasisPoints(500))
        .unwrap();
    db.category_archive(extra.id, ym("2026-11"), now()).unwrap();
    let rates: Vec<_> = db
        .savings_rates()
        .unwrap()
        .into_iter()
        .filter(|r| r.category_id == extra.id)
        .map(|r| (r.valid_from, r.rate))
        .collect();
    assert_eq!(
        rates,
        vec![
            (ym("2026-10"), BasisPoints(500)),
            (ym("2026-11"), BasisPoints(0))
        ]
    );
}

#[test]
fn last_savings_category_cannot_be_archived_while_the_year_has_savings() {
    let (_dir, mut db) = seeded();
    let id = id_of(&db, "Сбережения");
    insert_tx(&db, id, "2026-03");
    assert!(matches!(
        db.category_archive(id, ym("2026-10"), now()),
        Err(StorageError::Conflict("category.last_savings"))
    ));
    db.category_archive(id, ym("2027-01"), now()).unwrap();
}

#[test]
fn delete_is_allowed_only_without_transactions() {
    let (_dir, mut db) = seeded();
    let used = id_of(&db, "Продукты");
    insert_tx(&db, used, "2026-09");
    assert!(matches!(
        db.category_delete(used),
        Err(StorageError::Conflict("category.in_use"))
    ));
    let fresh = db
        .category_create(&new("Кино", CategoryKind::Wants), now())
        .unwrap();
    db.limit_set(fresh.id, ym("2026-10"), rub(1000)).unwrap();
    db.category_delete(fresh.id).unwrap();
    assert!(matches!(db.category(fresh.id), Err(StorageError::NotFound)));
    let left: i64 = db
        .conn()
        .query_row(
            "SELECT count(*) FROM category_limits WHERE category_id = ?1",
            [fresh.id.0],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(left, 0);
}

#[test]
fn limit_history_upserts_and_clears() {
    let (_dir, mut db) = seeded();
    let id = id_of(&db, "Продукты");
    let seeded_len = db.limit_history(id).unwrap().len();
    db.limit_set(id, ym("2026-11"), rub(9000)).unwrap();
    db.limit_set(id, ym("2026-11"), rub(9500)).unwrap();
    let history = db.limit_history(id).unwrap();
    assert_eq!(history.len(), seeded_len + 1);
    let last = history.last().unwrap();
    assert_eq!(
        (last.valid_from, last.amount),
        (ym("2026-11"), Some(rub(9500)))
    );
    db.limit_clear(id, ym("2026-11")).unwrap();
    assert!(matches!(
        db.limit_clear(id, ym("2026-11")),
        Err(StorageError::NotFound)
    ));
    assert_eq!(db.limit_history(id).unwrap().len(), seeded_len);
}

#[test]
fn limit_unset_writes_a_no_limit_row_and_limit_set_overwrites_it() {
    let (_dir, mut db) = seeded();
    let id = id_of(&db, "Продукты");
    db.limit_set(id, ym("2026-10"), rub(9000)).unwrap();
    db.limit_unset(id, ym("2026-11")).unwrap();
    let last = db.limit_history(id).unwrap().pop().unwrap();
    assert_eq!((last.valid_from, last.amount), (ym("2026-11"), None));
    db.limit_unset(id, ym("2026-11")).unwrap();
    db.limit_set(id, ym("2026-11"), rub(100)).unwrap();
    assert_eq!(
        db.limit_history(id).unwrap().pop().unwrap().amount,
        Some(rub(100))
    );
    let savings = id_of(&db, "Сбережения");
    assert!(db.limit_unset(savings, ym("2026-11")).is_err());
}

#[test]
fn limit_rejects_negative_amount_and_savings_category() {
    let (_dir, mut db) = seeded();
    let food = id_of(&db, "Продукты");
    let savings = id_of(&db, "Сбережения");
    assert!(matches!(
        db.limit_set(food, ym("2026-10"), rub(-1)),
        Err(StorageError::Invalid("limit.negative"))
    ));
    assert!(matches!(
        db.limit_set(savings, ym("2026-10"), rub(1)),
        Err(StorageError::Invalid("limit.savings_has_rate"))
    ));
}

#[test]
fn savings_rate_and_override_roundtrip_with_validation() {
    let (_dir, mut db) = seeded();
    let savings = id_of(&db, "Сбережения");
    let food = id_of(&db, "Продукты");
    db.savings_rate_set(savings, ym("2026-12"), BasisPoints(1500))
        .unwrap();
    db.savings_override_set(ym("2026-12"), savings, BasisPoints(2000))
        .unwrap();
    db.savings_override_set(ym("2026-12"), savings, BasisPoints(2100))
        .unwrap();
    assert_eq!(
        db.savings_overrides()
            .unwrap()
            .get(&(ym("2026-12"), savings)),
        Some(&BasisPoints(2100))
    );
    db.savings_override_clear(ym("2026-12"), savings).unwrap();
    assert!(db.savings_overrides().unwrap().is_empty());
    assert!(matches!(
        db.savings_override_clear(ym("2026-12"), savings),
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        db.savings_rate_set(savings, ym("2026-12"), BasisPoints(10_001)),
        Err(StorageError::Invalid("savings.rate_out_of_range"))
    ));
    assert!(matches!(
        db.savings_rate_set(food, ym("2026-12"), BasisPoints(100)),
        Err(StorageError::Invalid("savings.not_a_savings_category"))
    ));
}

//! Репозиторий тегов: создание, переименование, удаление и имена тегов трат.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::{Money, TagId, TxStatus, YearMonth};
use planning_budget_storage::{Db, NewTransaction, RecordSource, StorageError};
use tempfile::TempDir;

const KEY: [u8; 32] = [6; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn spend(db: &mut Db, month: &str, title: &str) -> planning_budget_core::TxId {
    let category_id = db.categories(false).unwrap().first().unwrap().id;
    db.transaction_create(
        &NewTransaction {
            month: ym(month),
            date: None,
            category_id,
            title: title.to_owned(),
            amount: Money::from_kopecks(10_000),
            status: TxStatus::Paid,
            comment: None,
            source: RecordSource::Manual,
        },
        now(),
    )
    .unwrap()
    .id
}

#[test]
fn tags_are_listed_by_normalized_name() {
    let (_dir, mut db) = seeded();
    assert!(db.tags().unwrap().is_empty());
    db.tag_create("Ёлка").unwrap();
    db.tag_create("арбуз").unwrap();
    db.tag_create("Бананы").unwrap();

    let names: Vec<String> = db.tags().unwrap().into_iter().map(|t| t.name).collect();

    assert_eq!(names, ["арбуз", "Бананы", "Ёлка"]);
}

#[test]
fn name_is_trimmed_and_must_not_be_empty() {
    let (_dir, mut db) = seeded();
    assert_eq!(db.tag_create("  отпуск  ").unwrap().name, "отпуск");
    assert!(matches!(
        db.tag_create("   "),
        Err(StorageError::Invalid(_))
    ));
}

#[test]
fn duplicate_name_ignores_case_and_yo() {
    let (_dir, mut db) = seeded();
    db.tag_create("Ёлка").unwrap();
    for same in ["ёлка", "ЁЛКА", "елка"] {
        assert!(
            matches!(
                db.tag_create(same),
                Err(StorageError::Conflict("tag.name_taken"))
            ),
            "{same}"
        );
    }
}

#[test]
fn rename_changes_the_name_and_guards_conflicts() {
    let (_dir, mut db) = seeded();
    let a = db.tag_create("дача").unwrap();
    let b = db.tag_create("море").unwrap();

    let renamed = db.tag_rename(a.id, "Дача 2026").unwrap();
    assert_eq!(renamed.name, "Дача 2026");
    assert!(matches!(
        db.tag_rename(b.id, "дача 2026"),
        Err(StorageError::Conflict("tag.name_taken"))
    ));
    assert!(matches!(
        db.tag_rename(b.id, ""),
        Err(StorageError::Invalid(_))
    ));
    assert!(matches!(
        db.tag_rename(TagId(9_999), "нет такого"),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn delete_unlinks_the_tag_from_transactions() {
    let (_dir, mut db) = seeded();
    let tag = db.tag_create("поездка").unwrap();
    let tx = spend(&mut db, "2026-09", "билеты");
    db.transaction_tags_set(tx, &[tag.id]).unwrap();
    assert_eq!(db.transaction(tx).unwrap().tags, [tag.id]);

    db.tag_delete(tag.id).unwrap();

    assert!(db.transaction(tx).unwrap().tags.is_empty());
    assert!(db.tags().unwrap().is_empty());
    assert!(matches!(db.tag_delete(tag.id), Err(StorageError::NotFound)));
}

#[test]
fn tx_tag_names_cover_the_period_and_skip_deleted_transactions() {
    let (_dir, mut db) = seeded();
    let sea = db.tag_create("море").unwrap();
    let work = db.tag_create("Работа").unwrap();
    let in_range = spend(&mut db, "2026-08", "пляж");
    let out_of_range = spend(&mut db, "2026-05", "офис");
    let deleted = spend(&mut db, "2026-08", "удалённая");
    db.transaction_tags_set(in_range, &[work.id, sea.id])
        .unwrap();
    db.transaction_tags_set(out_of_range, &[sea.id]).unwrap();
    db.transaction_tags_set(deleted, &[sea.id]).unwrap();
    db.transaction_delete(deleted, now()).unwrap();

    let names = db.tx_tag_names(ym("2026-07"), ym("2026-09")).unwrap();

    assert_eq!(names.len(), 1);
    assert_eq!(names[&in_range], ["море", "Работа"]);
}

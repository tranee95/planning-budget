//! Репозиторий тегов: создание, переименование, удаление и имена тегов трат.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::{Money, TagId, TxStatus, YearMonth};
use planning_budget_storage::{Db, FilterScreen, NewTransaction, RecordSource, StorageError};
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

#[test]
fn usage_counts_live_transactions_and_stored_queries_per_tag() {
    let (_dir, mut db) = seeded();
    let trip = db.tag_create("поездка").unwrap();
    let gift = db.tag_create("подарок").unwrap();
    let unused = db.tag_create("пусто").unwrap();
    let a = spend(&mut db, "2026-09", "билеты");
    let b = spend(&mut db, "2026-10", "отель");
    let c = spend(&mut db, "2026-10", "сувенир");
    db.transaction_tags_set(a, &[trip.id]).unwrap();
    db.transaction_tags_set(b, &[trip.id, gift.id]).unwrap();
    db.transaction_tags_set(c, &[gift.id]).unwrap();
    db.transaction_delete(c, now()).unwrap();
    db.saved_filter_save("отпуск", "#поездка кат:еда", FilterScreen::Expenses, now())
        .unwrap();

    let usage = db.tags_usage().unwrap();
    let by_name = |n: &str| usage.iter().find(|u| u.tag.name == n).unwrap();

    assert_eq!((by_name("поездка").uses, by_name("поездка").refs), (2, 1));
    assert_eq!(
        (by_name("подарок").uses, by_name("подарок").refs),
        (1, 0),
        "a deleted transaction is not counted"
    );
    assert_eq!(by_name("пусто").tag.id, unused.id);
    assert_eq!((by_name("пусто").uses, by_name("пусто").refs), (0, 0));
}

#[test]
fn rename_rewrites_saved_filters_and_chart_filters() {
    let (_dir, mut db) = seeded();
    let tag = db.tag_create("поездка").unwrap();
    db.saved_filter_save(
        "отпуск",
        "кат:еда  -#Поездка #поездка",
        FilterScreen::Expenses,
        now(),
    )
    .unwrap();
    db.saved_filter_save("прочее", "кат:еда", FilterScreen::Expenses, now())
        .unwrap();
    let dashboard = db.dashboards().unwrap().first().unwrap().id;
    let mut spec = planning_budget_core::analytics::standard_dashboard()
        .first()
        .unwrap()
        .spec
        .clone();
    spec.filter = "#поездка".to_owned();
    let card = db.chart_create(dashboard, &spec, 6, 4, now()).unwrap();

    db.tag_rename(tag.id, "Дача и сад").unwrap();

    let filters = db.saved_filters().unwrap();
    let queries: Vec<&str> = filters.iter().map(|f| f.query.as_str()).collect();
    assert!(queries.contains(&"кат:еда  -#\"Дача и сад\" #\"Дача и сад\""));
    assert!(
        queries.contains(&"кат:еда"),
        "unrelated filters are untouched"
    );
    let card_after = db
        .charts(dashboard)
        .unwrap()
        .into_iter()
        .find(|c| c.id == card.id)
        .unwrap();
    assert_eq!(card_after.spec.filter, "#\"Дача и сад\"");
    assert_eq!(db.tags_usage().unwrap().first().unwrap().refs, 2);
}

#[test]
fn rename_to_a_taken_name_changes_nothing() {
    let (_dir, mut db) = seeded();
    let a = db.tag_create("поездка").unwrap();
    db.tag_create("дача").unwrap();
    db.saved_filter_save("отпуск", "#поездка", FilterScreen::Expenses, now())
        .unwrap();

    assert!(matches!(
        db.tag_rename(a.id, "ДАЧА"),
        Err(StorageError::Conflict(_))
    ));

    assert_eq!(
        db.saved_filters().unwrap().first().unwrap().query,
        "#поездка"
    );
    assert_eq!(db.tags().unwrap().first().unwrap().name, "дача");
}

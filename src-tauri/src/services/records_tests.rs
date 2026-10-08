//! Сервисы доходов, тегов и категорий без Tauri: жизненный цикл записей и ключи ошибок.

#![allow(clippy::unwrap_used, reason = "тесты")]

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use planning_budget_core::{CategoryId, IncomeId, TxId};
use planning_budget_storage::Db;
use tempfile::TempDir;

use super::{categories, devseed, records};
use crate::AppError;
use crate::dto::{
    CategoryInput, CategoryKindDto, CategoryPatchDto, IncomeInput, IncomePatchDto, IncomeStatusDto,
    TransactionInput, TxStatusDto,
};

const KEY: [u8; 32] = [9; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap()
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
}

fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    devseed::load(&mut db, now()).unwrap();
    (dir, db)
}

fn income_input(month: &str, amount: i64) -> IncomeInput {
    IncomeInput {
        month: month.into(),
        date: None,
        source_name: "Подработка".into(),
        amount,
        status: IncomeStatusDto::Expected,
        comment: None,
    }
}

fn new_category(db: &mut Db, name: &str, kind: CategoryKindDto) -> i64 {
    categories::create(
        db,
        CategoryInput {
            name: name.into(),
            kind,
            color: "#3AA567".into(),
            note: None,
        },
        now(),
    )
    .unwrap()
    .id
}

#[test]
fn income_lifecycle_create_update_delete_restore() {
    let (_dir, mut db) = seeded();
    let before = records::incomes_list(&db, "2026-09").unwrap().len();

    let created = records::income_create(&mut db, income_input("2026-09", 500_000), now()).unwrap();
    assert_eq!(
        records::incomes_list(&db, "2026-09").unwrap().len(),
        before + 1
    );

    let id = IncomeId(created.id);
    let updated = records::income_update(
        &mut db,
        id,
        IncomePatchDto {
            amount: Some(750_000),
            status: Some(IncomeStatusDto::Received),
            month: Some("2026-10".into()),
            ..IncomePatchDto::default()
        },
        now(),
    )
    .unwrap();
    assert_eq!(updated.previous.month, "2026-09");
    assert_eq!(updated.previous.amount, 500_000);
    assert_eq!(updated.current.month, "2026-10");
    assert_eq!(updated.current.amount, 750_000);
    assert_eq!(updated.current.status, IncomeStatusDto::Received);

    let deleted = records::income_delete(&mut db, id, now()).unwrap();
    assert_eq!(deleted.id, created.id);
    assert!(
        records::incomes_list(&db, "2026-10")
            .unwrap()
            .iter()
            .all(|i| i.id != created.id)
    );
    let restored = records::income_restore(&mut db, id, now()).unwrap();
    assert_eq!(restored.amount, 750_000);
}

#[test]
fn income_errors_carry_keys() {
    let (_dir, mut db) = seeded();

    let err = records::incomes_list(&db, "2026-13").unwrap_err();
    assert!(matches!(
        err,
        AppError::Validation { ref message_key, field: Some(ref f) }
            if message_key == "errors.month_invalid" && f == "month"
    ));

    let err = records::income_create(&mut db, income_input("2026-09", 0), now()).unwrap_err();
    assert!(matches!(
        err,
        AppError::Validation { ref message_key, .. } if message_key == "errors.record.amount_not_positive"
    ));

    let err = records::income_delete(&mut db, IncomeId(999_999), now()).unwrap_err();
    assert!(matches!(
        err,
        AppError::NotFound { ref entity, id: 999_999 } if entity == "income"
    ));
    let err = records::income_restore(&mut db, IncomeId(999_999), now()).unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[test]
fn tags_are_created_listed_and_attached_to_a_transaction() {
    let (_dir, mut db) = seeded();
    let food = db.category_id_by_name("Продукты").unwrap().unwrap();
    let tx = records::tx_create(
        &mut db,
        TransactionInput {
            month: "2026-09".into(),
            date: None,
            category_id: food.0,
            title: "Рынок".into(),
            amount: 100_000,
            status: TxStatusDto::Paid,
            comment: None,
        },
        now(),
    )
    .unwrap();
    let tag = records::tag_create(&mut db, "дача").unwrap();
    assert_eq!(
        records::tags_list(&db)
            .unwrap()
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["дача"]
    );

    records::tx_tags_set(&mut db, TxId(tx.id), vec![tag.id]).unwrap();
    assert_eq!(
        db.transaction(TxId(tx.id)).unwrap().tags,
        [planning_budget_core::TagId(tag.id)]
    );
    records::tx_tags_set(&mut db, TxId(tx.id), Vec::new()).unwrap();
    assert!(db.transaction(TxId(tx.id)).unwrap().tags.is_empty());

    let err = records::tag_create(&mut db, "ДАЧА").unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }));
    let err = records::tx_tags_set(&mut db, TxId(999_999), vec![tag.id]).unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[test]
fn touched_months_lists_both_months_only_when_they_differ() {
    assert_eq!(records::touched_months("2026-09", "2026-09"), ["2026-09"]);
    assert_eq!(
        records::touched_months("2026-09", "2026-10"),
        ["2026-09", "2026-10"]
    );
}

#[test]
fn category_archive_unarchive_and_delete() {
    let (_dir, mut db) = seeded();
    let id = new_category(&mut db, "Хобби", CategoryKindDto::Wants);

    categories::archive(&mut db, CategoryId(id), today(), now()).unwrap();
    assert!(
        categories::list(&db, false)
            .unwrap()
            .iter()
            .all(|c| c.id != id)
    );
    assert!(
        categories::list(&db, true)
            .unwrap()
            .iter()
            .any(|c| c.id == id && c.archived)
    );

    categories::unarchive(&mut db, CategoryId(id), now()).unwrap();
    assert!(
        categories::list(&db, false)
            .unwrap()
            .iter()
            .any(|c| c.id == id)
    );

    categories::delete(&mut db, CategoryId(id)).unwrap();
    assert!(
        categories::list(&db, true)
            .unwrap()
            .iter()
            .all(|c| c.id != id)
    );
    let err = categories::delete(&mut db, CategoryId(id)).unwrap_err();
    assert!(matches!(
        err,
        AppError::NotFound { ref entity, .. } if entity == "category"
    ));
}

#[test]
fn reorder_returns_the_new_order_and_rejects_foreign_ids() {
    let (_dir, mut db) = seeded();
    let mut ids: Vec<i64> = categories::list(&db, false)
        .unwrap()
        .iter()
        .map(|c| c.id)
        .collect();
    ids.reverse();

    let listed = categories::reorder(&mut db, ids.clone(), now()).unwrap();

    assert_eq!(listed.iter().map(|c| c.id).collect::<Vec<_>>(), ids);

    ids.push(999_999);
    assert!(categories::reorder(&mut db, ids, now()).is_err());
}

#[test]
fn limits_are_set_cleared_and_listed_in_history() {
    let (_dir, mut db) = seeded();
    let id = CategoryId(new_category(&mut db, "Хобби", CategoryKindDto::Wants));

    categories::limits_set(&mut db, id, "2026-05", 300_000).unwrap();
    categories::limits_set(&mut db, id, "2026-08", 450_000).unwrap();
    categories::limits_unset(&mut db, id, "2026-10").unwrap();
    let history = categories::limits_history(&db, id).unwrap();
    let rows: Vec<(&str, Option<i64>)> = history
        .iter()
        .map(|e| (e.valid_from.as_str(), e.amount))
        .collect();
    assert_eq!(
        rows,
        [
            ("2026-05", Some(300_000)),
            ("2026-08", Some(450_000)),
            ("2026-10", None)
        ]
    );

    categories::limits_clear(&mut db, id, "2026-08").unwrap();
    assert_eq!(categories::limits_history(&db, id).unwrap().len(), 2);

    for bad in ["2026-13", "oops"] {
        assert!(matches!(
            categories::limits_unset(&mut db, id, bad).unwrap_err(),
            AppError::Validation { .. }
        ));
        assert!(matches!(
            categories::limits_clear(&mut db, id, bad).unwrap_err(),
            AppError::Validation { .. }
        ));
    }
}

#[test]
fn savings_rate_and_month_override_are_set_and_cleared() {
    let (_dir, mut db) = seeded();
    let savings = categories::list(&db, false)
        .unwrap()
        .into_iter()
        .find(|c| c.kind == CategoryKindDto::Savings)
        .unwrap();
    let id = CategoryId(savings.id);

    categories::savings_rate_set(&mut db, id, "2026-09", 1500).unwrap();
    let set = categories::savings_override_set(&mut db, "2026-09", id, 2000).unwrap();
    assert_eq!(set.to_string(), "2026-09");
    let cleared = categories::savings_override_clear(&mut db, "2026-09", id).unwrap();
    assert_eq!(cleared.to_string(), "2026-09");

    assert!(matches!(
        categories::savings_rate_set(&mut db, id, "2026-09", 10_001).unwrap_err(),
        AppError::Validation { .. }
    ));
    assert!(matches!(
        categories::savings_override_set(&mut db, "2026-14", id, 100).unwrap_err(),
        AppError::Validation { .. }
    ));
}

#[test]
fn category_update_changes_the_name_and_keeps_the_kind() {
    let (_dir, mut db) = seeded();
    let id = new_category(&mut db, "Хобби", CategoryKindDto::Wants);

    let updated = categories::update(
        &mut db,
        CategoryId(id),
        CategoryPatchDto {
            name: Some("Увлечения".into()),
            ..CategoryPatchDto::default()
        },
        now(),
    )
    .unwrap();

    assert_eq!(updated.name, "Увлечения");
    assert_eq!(updated.kind, CategoryKindDto::Wants);
}

#[test]
fn dashboards_and_cards_round_trip_through_the_service() {
    use planning_budget_core::analytics::standard_dashboard;

    use super::analytics;
    use crate::dto::{CardPlacementDto, Int53};

    let (_dir, mut db) = seeded();
    let spec = analytics::spec_to_dto(&standard_dashboard().swap_remove(0).spec).unwrap();

    // Сид уже создал стандартный дашборд: второй раз он не добавляется.
    let first = analytics::dashboards_list(&db).unwrap().swap_remove(0);
    assert!(first.is_default);
    assert!(matches!(
        analytics::dashboard_create_default(&mut db, now()).unwrap_err(),
        AppError::Conflict { ref message_key } if message_key == "errors.dashboard.exists"
    ));
    let second = analytics::dashboard_create(&mut db, "Отпуск").unwrap();
    analytics::dashboard_rename(&mut db, second.id, "Поездка").unwrap();
    let names: Vec<String> = analytics::dashboards_list(&db)
        .unwrap()
        .into_iter()
        .map(|d| d.name)
        .collect();
    assert!(names.contains(&"Поездка".to_owned()), "{names:?}");

    let card = analytics::chart_create(&mut db, second.id, &spec, 6, 4, now()).unwrap();
    assert_eq!((card.w, card.h), (6, 4));
    let mut changed = spec.clone();
    changed.title = "Новое название".into();
    let updated = analytics::chart_update(&mut db, card.id, &changed, now()).unwrap();
    assert_eq!(updated.spec.title, "Новое название");

    analytics::charts_layout_set(
        &mut db,
        second.id,
        &[CardPlacementDto {
            id: Int53(card.id),
            x: 6,
            y: 0,
            w: 6,
            h: 4,
        }],
        now(),
    )
    .unwrap();
    let cards = analytics::charts_list(&db, second.id).unwrap();
    assert_eq!((cards.len(), cards[0].x), (1, 6));

    analytics::chart_delete(&mut db, card.id).unwrap();
    assert!(analytics::charts_list(&db, second.id).unwrap().is_empty());
    analytics::dashboard_delete(&mut db, second.id).unwrap();
    assert!(matches!(
        analytics::dashboard_delete(&mut db, first.id).unwrap_err(),
        AppError::Validation { .. }
    ));
}

#[test]
fn storage_and_import_errors_map_to_app_error_keys() {
    use planning_budget_import::ImportError;
    use planning_budget_storage::StorageError;

    assert!(matches!(
        AppError::from(StorageError::Invalid("record.title_empty")),
        AppError::Validation { ref message_key, field: None } if message_key == "errors.record.title_empty"
    ));
    assert!(matches!(
        AppError::from(StorageError::NotFound),
        AppError::NotFound { ref entity, id: 0 } if entity == "record"
    ));
    assert!(matches!(
        AppError::from_storage(StorageError::NotFound, "debt", 7),
        AppError::NotFound { ref entity, id: 7 } if entity == "debt"
    ));
    assert!(matches!(
        AppError::from_storage(StorageError::Conflict("category.name_taken"), "debt", 7),
        AppError::Conflict { ref message_key } if message_key == "errors.category.name_taken"
    ));
    assert!(matches!(
        AppError::invalid_field("month_invalid", "month"),
        AppError::Validation { ref message_key, field: Some(ref f) }
            if message_key == "errors.month_invalid" && f == "month"
    ));

    let keyed = |e: ImportError| match AppError::from(e) {
        AppError::Import { message_key, line } => (message_key, line),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        keyed(ImportError::NotXlsx),
        ("errors.import.legacy.not_xlsx".to_owned(), None)
    );
    assert_eq!(
        keyed(ImportError::NoYear),
        ("errors.import.legacy.no_year".to_owned(), None)
    );
    assert_eq!(
        keyed(ImportError::UnknownKind { row: 12 }),
        ("errors.import.legacy.unknown_kind".to_owned(), Some(12))
    );
}

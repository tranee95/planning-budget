//! Дашборды и карточки графиков.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::analytics::{ChartSpec, ChartType, standard_dashboard};
use planning_budget_storage::{CardPlacement, Db, StorageError};
use tempfile::TempDir;

const KEY: [u8; 32] = [5; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn spec() -> ChartSpec {
    standard_dashboard().swap_remove(0).spec
}

/// Ключ ошибки `Invalid`; любая другая ошибка даёт «other», и сравнение в тесте падает.
fn invalid_key(err: StorageError) -> &'static str {
    match err {
        StorageError::Invalid(key) => key,
        _ => "other",
    }
}

#[test]
fn seeded_dashboard_is_listed_with_its_cards_in_reading_order() {
    let (_dir, db) = seeded();
    let dashboards = db.dashboards().unwrap();
    assert_eq!(dashboards.len(), 1);
    assert!(dashboards[0].is_default);
    let cards = db.charts(dashboards[0].id).unwrap();
    assert_eq!(cards.len(), 9);
    assert_eq!((cards[0].x, cards[0].y), (0, 0));
    assert_eq!((cards[1].x, cards[1].y), (6, 0));
}

#[test]
fn new_card_goes_below_the_others_and_update_keeps_the_place() {
    let (_dir, mut db) = seeded();
    let id = db.dashboards().unwrap()[0].id;
    let card = db.chart_create(id, &spec(), 6, 4, now()).unwrap();
    assert_eq!((card.x, card.y, card.w, card.h), (0, 25, 6, 4));

    let mut edited = spec();
    edited.title = "Другое название".to_owned();
    edited.chart_type = ChartType::Line;
    let updated = db.chart_update(card.id, &edited, now()).unwrap();
    assert_eq!(updated.spec.title, "Другое название");
    assert_eq!((updated.x, updated.y), (0, 25));
}

#[test]
fn invalid_specs_and_sizes_are_rejected_with_a_key() {
    let (_dir, mut db) = seeded();
    let id = db.dashboards().unwrap()[0].id;
    let mut bad = spec();
    bad.chart_type = ChartType::StackedBar;
    bad.series_by = None;
    bad.metrics.clear();
    assert_eq!(
        invalid_key(db.chart_create(id, &bad, 6, 4, now()).unwrap_err()),
        "chart.stacked_without_series"
    );
    assert_eq!(
        invalid_key(db.chart_create(id, &spec(), 13, 4, now()).unwrap_err()),
        "dashboard.card_size"
    );
    assert!(matches!(
        db.chart_create(999, &spec(), 6, 4, now()),
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        db.chart_update(999, &spec(), now()),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn layout_is_saved_atomically_and_checked() {
    let (_dir, mut db) = seeded();
    let id = db.dashboards().unwrap()[0].id;
    let cards = db.charts(id).unwrap();
    let first = cards[0].id;
    let second = cards[1].id;

    db.charts_layout_set(
        id,
        &[
            CardPlacement {
                id: first,
                x: 6,
                y: 0,
                w: 6,
                h: 4,
            },
            CardPlacement {
                id: second,
                x: 0,
                y: 0,
                w: 6,
                h: 4,
            },
        ],
        now(),
    )
    .unwrap();
    let after = db.charts(id).unwrap();
    assert_eq!(after[0].id, second);
    assert_eq!(after[1].id, first);

    let off_grid = [CardPlacement {
        id: first,
        x: 8,
        y: 0,
        w: 6,
        h: 4,
    }];
    assert_eq!(
        invalid_key(db.charts_layout_set(id, &off_grid, now()).unwrap_err()),
        "dashboard.layout"
    );
    let twice = [
        CardPlacement {
            id: first,
            x: 0,
            y: 0,
            w: 6,
            h: 4,
        },
        CardPlacement {
            id: first,
            x: 6,
            y: 0,
            w: 6,
            h: 4,
        },
    ];
    assert_eq!(
        invalid_key(db.charts_layout_set(id, &twice, now()).unwrap_err()),
        "dashboard.layout"
    );

    let other = db.dashboard_create("Второй").unwrap();
    let foreign = [CardPlacement {
        id: first,
        x: 0,
        y: 0,
        w: 6,
        h: 4,
    }];
    assert_eq!(
        invalid_key(db.charts_layout_set(other.id, &foreign, now()).unwrap_err()),
        "dashboard.layout"
    );
    assert_eq!(
        db.charts(id).unwrap()[0].id,
        second,
        "неудачный вызов ничего не менял"
    );
}

#[test]
fn dashboards_are_created_renamed_and_deleted_with_their_cards() {
    let (_dir, mut db) = seeded();
    let main = db.dashboards().unwrap()[0].clone();
    let second = db.dashboard_create("  Отпуск ").unwrap();
    assert_eq!(second.name, "Отпуск");
    assert!(!second.is_default);
    db.chart_create(second.id, &spec(), 12, 4, now()).unwrap();

    db.dashboard_rename(second.id, "Поездки").unwrap();
    assert_eq!(db.dashboards().unwrap()[1].name, "Поездки");
    assert_eq!(
        invalid_key(db.dashboard_create("   ").unwrap_err()),
        "dashboard.name_empty"
    );

    db.dashboard_delete(second.id).unwrap();
    assert!(matches!(db.charts(second.id).map(|c| c.len()), Ok(0)));
    assert_eq!(db.dashboards().unwrap().len(), 1);
    assert_eq!(
        invalid_key(db.dashboard_delete(main.id).unwrap_err()),
        "dashboard.last"
    );
}

#[test]
fn deleting_the_default_dashboard_promotes_the_first_remaining() {
    let (_dir, mut db) = seeded();
    let main = db.dashboards().unwrap()[0].id;
    let second = db.dashboard_create("Второй").unwrap();
    db.dashboard_delete(main).unwrap();
    let left = db.dashboards().unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, second.id);
    assert!(left[0].is_default);
}

#[test]
fn deleting_a_card_removes_only_that_card() {
    let (_dir, mut db) = seeded();
    let id = db.dashboards().unwrap()[0].id;
    let first = db.charts(id).unwrap()[0].id;
    db.chart_delete(first).unwrap();
    assert_eq!(db.charts(id).unwrap().len(), 8);
    assert!(matches!(
        db.chart_delete(first),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn a_dashboard_that_would_outgrow_the_row_limit_refuses_new_cards() {
    let (_dir, mut db) = seeded();
    let id = db.dashboards().unwrap()[0].id;
    let mut added = 0;
    let key = loop {
        match db.chart_create(id, &spec(), 12, 12, now()) {
            Ok(_) => added += 1,
            Err(e) => break invalid_key(e),
        }
        assert!(added < 100, "лимит не сработал");
    };
    assert_eq!(key, "dashboard.full");
    // дашборд по-прежнему читается целиком
    assert_eq!(db.charts(id).unwrap().len(), 9 + added);
}

#[test]
fn standard_dashboard_is_offered_only_when_none_exist_and_only_once() {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    let first = db.dashboards().unwrap().remove(0);
    db.dashboard_create("Свой").unwrap();
    db.dashboard_delete(first.id).unwrap();
    // Повторный сид удалённый дашборд не возвращает.
    db.seed_defaults(now()).unwrap();
    assert_eq!(db.dashboards().unwrap().len(), 1);

    let mut empty = Db::create(&dir.path().join("empty.db"), &KEY).unwrap();
    assert!(empty.dashboards().unwrap().is_empty());
    empty.seed_standard_dashboard(now()).unwrap();
    let list = empty.dashboards().unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].is_default);
    assert_eq!(
        empty.charts(list[0].id).unwrap().len(),
        standard_dashboard().len()
    );

    let err = empty.seed_standard_dashboard(now()).unwrap_err();
    assert!(matches!(err, StorageError::Conflict("dashboard.exists")));
    assert_eq!(empty.dashboards().unwrap().len(), 1);
}

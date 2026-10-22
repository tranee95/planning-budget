//! Сервисы данных без Tauri: сид → база → сводки совпадают с golden, ошибки с ключами.

#![allow(clippy::unwrap_used, reason = "тесты")]

use super::test_support::*;
use super::{analytics, categories, records, settings, summary};
use crate::AppError;
use crate::dto::{
    CategoryInput, CategoryKindDto, CategoryPatchDto, FilterScreenDto, SettingsPatchDto,
    TransactionPatchDto,
};
use planning_budget_core::{CategoryId, TxId};
use planning_budget_storage::Db;
use tempfile::TempDir;

#[test]
fn storage_errors_become_keyed_app_errors() {
    let (_dir, mut db) = seeded();
    let food = category(&db, "Продукты");

    let err = records::tx_delete(&mut db, TxId(999_999), now()).unwrap_err();
    assert!(matches!(
        err,
        AppError::NotFound { ref entity, id: 999_999 } if entity == "transaction"
    ));

    let err = categories::create(
        &mut db,
        CategoryInput {
            name: "продукты".into(),
            kind: CategoryKindDto::Wants,
            color: "#3AA567".into(),
            note: None,
        },
        now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        AppError::Conflict { ref message_key } if message_key == "errors.category.name_taken"
    ));

    let err = records::tx_update(
        &mut db,
        TxId(1),
        TransactionPatchDto {
            amount: Some(0),
            ..TransactionPatchDto::default()
        },
        now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        AppError::Validation { ref message_key, .. } if message_key == "errors.record.amount_not_positive"
    ));

    let err = categories::limits_set(&mut db, food, "2026-13", 100).unwrap_err();
    assert!(matches!(
        err,
        AppError::Validation { ref message_key, field: Some(ref f) }
            if message_key == "errors.month_invalid" && f == "validFrom"
    ));
}

#[test]
fn patch_distinguishes_missing_and_null_note() {
    let (_dir, mut db) = seeded();
    let food = category(&db, "Продукты");
    let set: CategoryPatchDto = serde_json::from_str(r#"{"note":"еда"}"#).unwrap();
    assert_eq!(
        categories::update(&mut db, food, set, now())
            .unwrap()
            .note
            .as_deref(),
        Some("еда")
    );
    let untouched: CategoryPatchDto = serde_json::from_str(r##"{"color":"#5B7FD6"}"##).unwrap();
    assert_eq!(
        categories::update(&mut db, food, untouched, now())
            .unwrap()
            .note
            .as_deref(),
        Some("еда")
    );
    let cleared: CategoryPatchDto = serde_json::from_str(r#"{"note":null}"#).unwrap();
    assert_eq!(
        categories::update(&mut db, food, cleared, now())
            .unwrap()
            .note,
        None
    );
}

#[test]
fn settings_patch_writes_only_given_fields() {
    let (_dir, mut db) = seeded();
    let before = settings::get(&db).unwrap();
    let after = settings::set(
        &mut db,
        SettingsPatchDto {
            autolock_minutes: Some(15),
            ..SettingsPatchDto::default()
        },
    )
    .unwrap();
    assert_eq!(after.autolock_minutes, 15);
    assert_eq!(after.savings_norm_bp, before.savings_norm_bp);
    let err = settings::set(
        &mut db,
        SettingsPatchDto {
            weeks_per_month: Some(9),
            ..SettingsPatchDto::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::Validation { .. }));
}

#[test]
fn search_finds_seed_records_and_reports_token_spans() {
    let (_dir, db) = seeded();
    let found = super::search::run(&db, "супермаркет статус:оплачено сумма>100", 7, 2026).unwrap();
    assert_eq!(found.request_id, 7);
    assert!(found.transactions.total > 0);
    assert!(found.transactions.items.len() <= 8);
    assert!(
        found
            .transactions
            .items
            .iter()
            .all(|t| t.title.to_lowercase().contains("супермаркет") && t.amount > 10_000)
    );
    let spans: Vec<_> = found.spans.iter().map(|s| (s.start, s.end)).collect();
    assert_eq!(spans, [(0, 11), (12, 27), (28, 37)]);
    assert!(found.hints.is_empty());

    let unknown = super::search::run(&db, "статус:абракадабра", 8, 2026).unwrap();
    assert_eq!(unknown.hints.len(), 1);
}

#[test]
fn filtered_lists_match_palette_totals_and_saved_filters_roundtrip() {
    let (_dir, mut db) = seeded();
    let palette = super::search::run(&db, "лента статус:оплачено", 1, 2026).unwrap();
    let list = super::search::tx_list(&db, "лента статус:оплачено", 2, 2026).unwrap();
    assert_eq!(list.request_id, 2);
    assert_eq!(list.total, palette.transactions.total);
    assert_eq!(list.sum, palette.transactions.sum);
    assert_eq!(list.items.len() as u64, list.total);
    assert!(!list.truncated);

    let incomes = super::search::income_list(&db, "доход", 3, 2026).unwrap();
    assert_eq!(incomes.items.len() as u64, incomes.total);

    let saved = super::search::filter_save(
        &mut db,
        "Лента",
        "лента статус:оплачено",
        FilterScreenDto::Expenses,
        now(),
    )
    .unwrap();
    let all = super::search::filters_list(&db).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, saved.id);
    super::search::filter_delete(&mut db, saved.id).unwrap();
    assert!(super::search::filters_list(&db).unwrap().is_empty());

    let suggestions = super::search::suggest(&db, "лен").unwrap();
    assert!(
        suggestions
            .iter()
            .all(|s| s.title.to_lowercase().contains("лен"))
    );
    assert!(
        !super::search::category_usage(&db, today())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn overlong_queries_are_clipped_before_parsing() {
    let (_dir, db) = seeded();
    let long = "супермаркет ".repeat(1_000);
    let found = super::search::run(&db, &long, 1, 2026).unwrap();
    assert!(found.spans.iter().all(|s| s.end <= 500));
    let list = super::search::tx_list(&db, &long, 2, 2026).unwrap();
    assert!(list.spans.iter().all(|s| s.end <= 500));
}

#[test]
fn analytics_run_uses_the_database_and_reports_invalid_specs_by_key() {
    use planning_budget_core::analytics::standard_dashboard;

    let (_dir, db) = seeded();
    let placement = standard_dashboard().swap_remove(0);
    let mut dto = analytics::spec_to_dto(&placement.spec).unwrap();
    let chart = analytics::run(&db, &dto, today()).unwrap();
    assert_eq!(chart.categories.len(), 9);
    let income = chart.series.iter().find(|s| s.name == "Доходы").unwrap();
    assert!((income.values[8].unwrap() - 163_000.0).abs() < 0.01);

    dto.metrics.clear();
    dto.series_by = None;
    dto.chart_type = crate::dto::ChartTypeDto::StackedBar;
    let AppError::Validation { message_key, .. } = analytics::run(&db, &dto, today()).unwrap_err()
    else {
        panic!("ожидалась ошибка проверки");
    };
    assert_eq!(message_key, "errors.chart.stacked_without_series");

    // Неверный месяц периода: ключ и поле берутся из разбора описания, как до B2.
    let mut bad = analytics::spec_to_dto(&placement.spec).unwrap();
    bad.period = crate::dto::ChartPeriodDto::Range {
        from: "2026-13".to_owned(),
        to: "2026-12".to_owned(),
    };
    let AppError::Validation { message_key, field } =
        analytics::run(&db, &bad, today()).unwrap_err()
    else {
        panic!("ожидалась ошибка проверки");
    };
    assert_eq!(message_key, "errors.month_invalid");
    assert_eq!(field.as_deref(), Some("period"));
}

#[test]
fn analytics_run_many_equals_single_runs_and_isolates_invalid_specs() {
    use planning_budget_core::analytics::standard_dashboard;

    let (_dir, db) = seeded();
    let mut specs: Vec<_> = standard_dashboard()
        .iter()
        .map(|p| analytics::spec_to_dto(&p.spec).unwrap())
        .collect();
    let mut narrow = specs[0].clone();
    narrow.period = crate::dto::ChartPeriodDto::Preset {
        preset: crate::dto::PeriodPresetDto::CurrentMonth,
    };
    let mut compare = specs[0].clone();
    compare.options.compare_prev_period = true;
    compare.period = crate::dto::ChartPeriodDto::Range {
        from: "2026-05".to_owned(),
        to: "2026-07".to_owned(),
    };
    let mut all = specs[0].clone();
    all.period = crate::dto::ChartPeriodDto::Preset {
        preset: crate::dto::PeriodPresetDto::All,
    };
    let mut invalid = specs[0].clone();
    invalid.metrics.clear();
    invalid.series_by = None;
    invalid.chart_type = crate::dto::ChartTypeDto::StackedBar;
    specs.extend([narrow, compare, all, invalid]);

    let many = analytics::run_many(&db, &specs, today()).unwrap();
    assert_eq!(many.len(), specs.len());
    for (spec, got) in specs.iter().zip(&many) {
        match analytics::run(&db, spec, today()) {
            Ok(single) => {
                assert!(got.error_key.is_none());
                assert_eq!(
                    serde_json::to_value(got.data.as_ref().unwrap()).unwrap(),
                    serde_json::to_value(&single).unwrap()
                );
            }
            Err(AppError::Validation { message_key, .. }) => {
                assert!(got.data.is_none());
                assert_eq!(got.error_key.as_deref(), Some(message_key.as_str()));
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    assert_eq!(
        many.last().unwrap().error_key.as_deref(),
        Some("errors.chart.stacked_without_series")
    );
    assert!(analytics::run_many(&db, &[], today()).unwrap().is_empty());
}

#[test]
fn limits_unset_removes_the_limit_from_that_month_only() {
    let (_dir, mut db) = seeded();
    let food = categories::list(&db, false)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Продукты")
        .unwrap();
    let before = summary::month(&db, "2026-09", today()).unwrap();
    let row = |o: &crate::dto::MonthOverviewDto, month_row: i64| {
        o.limits
            .iter()
            .find(|r| r.category_id == month_row)
            .unwrap()
            .limit
    };
    assert!(row(&before, food.id).is_some());

    categories::limits_unset(&mut db, CategoryId(food.id), "2026-09").unwrap();
    assert!(
        categories::limits_history(&db, CategoryId(food.id))
            .unwrap()
            .iter()
            .any(|e| e.valid_from == "2026-09" && e.amount.is_none())
    );
    let after = summary::month(&db, "2026-09", today()).unwrap();
    assert_eq!(row(&after, food.id), None);
    let earlier = summary::month(&db, "2026-08", today()).unwrap();
    assert!(row(&earlier, food.id).is_some());
}

#[test]
fn first_run_seed_creates_only_chosen_and_required_categories_once() {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_base(now()).unwrap();
    assert!(categories::list(&db, false).unwrap().is_empty());

    let defaults = categories::defaults().unwrap();
    assert_eq!(defaults.len(), 17);
    assert_eq!(defaults.iter().filter(|c| c.required).count(), 2);

    let chosen = vec!["Продукты".to_owned()];
    let (created, added) = categories::seed(&mut db, &chosen, now()).unwrap();
    assert_eq!(added, 3);
    let names: Vec<&str> = created.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Продукты", "Прочее", "Сбережения"]);

    // Повтор не дублирует и не добавляет снятое.
    let (again, added) = categories::seed(&mut db, &["Одежда".to_owned()], now()).unwrap();
    assert_eq!((again.len(), added), (3, 0));
}

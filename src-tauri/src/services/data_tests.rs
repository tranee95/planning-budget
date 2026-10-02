//! Сервисы данных без Tauri: сид → база → сводки совпадают с golden, ошибки с ключами.

#![allow(clippy::unwrap_used, reason = "тесты")]

use budget_core::{CategoryId, TxId};
use budget_storage::Db;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use tempfile::TempDir;

use super::{analytics, bonds, categories, devseed, records, settings, summary};
use crate::AppError;
use crate::dto::{
    CategoryInput, CategoryKindDto, CategoryPatchDto, SettingsPatchDto, TransactionInput,
    TransactionPatchDto, TxStatusDto,
};

const KEY: [u8; 32] = [7; 32];

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

fn category(db: &Db, name: &str) -> CategoryId {
    db.category_id_by_name(name).unwrap().unwrap()
}

fn rub(kopecks: i64) -> i64 {
    kopecks * 100
}

#[test]
fn month_overview_matches_golden_for_september() {
    let (_dir, db) = seeded();
    let overview = summary::month(&db, "2026-09").unwrap();
    let s = &overview.summary;
    assert_eq!(s.income, rub(163_000));
    assert_eq!(s.expenses, rub(99_027));
    assert_eq!(s.savings, 2_124_951);
    assert_eq!(s.free, 4_272_349);
    assert_eq!(s.by_status.planned, 3_144_851);
    assert_eq!(s.by_kind.mandatory, rub(63_819));
    assert_eq!(overview.limits.len(), 17);
}

#[test]
fn year_summary_matches_golden_and_ignores_future_months() {
    let (_dir, db) = seeded();
    let year = summary::year(&db, 2026, today()).unwrap();
    assert_eq!(year.months.len(), 12);
    assert_eq!(year.months_with_data, 9);
    assert_eq!(year.income, rub(1_330_000));
    assert_eq!(year.expenses, rub(924_091));
    assert_eq!(year.balance.limits_sum, rub(101_700));
    assert_eq!(year.balance.buffer, 2_538_889);
    assert_eq!(year.categories.len(), 17);

    let early = summary::year(&db, 2026, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()).unwrap();
    assert_eq!(early.months_with_data, 3);
    assert!(early.income < year.income);
}

#[test]
fn writes_change_the_summary_and_undo_restores_it() {
    let (_dir, mut db) = seeded();
    let food = category(&db, "Продукты");
    let before = summary::month(&db, "2026-09").unwrap().summary.expenses;
    let created = records::tx_create(
        &mut db,
        TransactionInput {
            month: "2026-09".into(),
            date: Some("2026-09-20".into()),
            category_id: food.0,
            title: "Лента".into(),
            amount: rub(1000),
            status: TxStatusDto::Paid,
            comment: None,
        },
        now(),
    )
    .unwrap();
    assert_eq!(
        summary::month(&db, "2026-09").unwrap().summary.expenses,
        before + rub(1000)
    );

    let updated =
        records::tx_set_status(&mut db, TxId(created.id), TxStatusDto::Planned, now()).unwrap();
    assert_eq!(updated.previous.status, TxStatusDto::Paid);

    records::tx_delete(&mut db, TxId(created.id), now()).unwrap();
    assert_eq!(
        summary::month(&db, "2026-09").unwrap().summary.expenses,
        before
    );
    records::tx_restore(&mut db, TxId(created.id), now()).unwrap();
    assert_eq!(
        summary::month(&db, "2026-09").unwrap().summary.expenses,
        before + rub(1000)
    );
}

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
fn two_savings_categories_split_the_plan() {
    let (_dir, mut db) = seeded();
    let reserve = categories::create(
        &mut db,
        CategoryInput {
            name: "Подушка".into(),
            kind: CategoryKindDto::Savings,
            color: "#5B7FD6".into(),
            note: None,
        },
        now(),
    )
    .unwrap();
    categories::savings_rate_set(&mut db, CategoryId(reserve.id), "2026-01", 500).unwrap();
    let year = summary::year(&db, 2026, today()).unwrap();
    let sep = year.months.iter().find(|m| m.month == "2026-09").unwrap();
    assert_eq!(sep.savings_plan_rate_bp, 1900);
    assert_eq!(sep.savings_plan, rub(163_000) * 19 / 100);
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

    let saved =
        super::search::filter_save(&mut db, "Лента", "лента статус:оплачено", now()).unwrap();
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
    use budget_core::analytics::standard_dashboard;

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
}

#[test]
fn bonds_projection_matches_golden() {
    let (_dir, db) = seeded();
    let dto = bonds::projection(&db, 2026, today()).unwrap();
    assert_eq!(dto.months.len(), 12);
    // golden: баланс на конец сентября 153 566,17, на конец декабря 158 652,00
    let september = dto.months.iter().find(|m| m.month == "2026-09").unwrap();
    assert!((september.balance - 15_356_617).abs() <= 100);
    assert!((dto.dec_balance - 15_865_200).abs() <= 100);
    assert!((dto.effective_rate - 0.1392).abs() < 1e-9);
    // golden: B на горизонте 5 лет — 2 414 468,03
    let b = dto
        .scenarios
        .iter()
        .find(|s| s.scenario == crate::dto::ForecastKindDto::B)
        .unwrap();
    assert!((b.y5 - 241_446_803).abs() <= 100);
    assert_eq!(b.balances.len(), 60);
    assert_eq!(b.balances.last().copied(), Some(b.y5));
}

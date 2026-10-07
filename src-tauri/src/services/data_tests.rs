//! Сервисы данных без Tauri: сид → база → сводки совпадают с golden, ошибки с ключами.

#![allow(clippy::unwrap_used, reason = "тесты")]

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use planning_budget_core::{CategoryId, DebtId, TxId};
use planning_budget_storage::Db;
use tempfile::TempDir;

use super::{analytics, categories, devseed, plan, records, savings, settings, summary};
use crate::AppError;
use crate::dto::{
    CategoryInput, CategoryKindDto, CategoryPatchDto, PlanBalanceDto, SettingsPatchDto,
    TransactionInput, TransactionPatchDto, TxStatusDto,
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
    let overview = summary::month(&db, "2026-09", today()).unwrap();
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
fn month_overview_carries_ready_percentages_for_the_ui() {
    let (_dir, db) = seeded();
    let overview = summary::month(&db, "2026-09", today()).unwrap();
    // Расходы сентября (99 027 ₽) против среднего за 9 месяцев (≈ 102 677 ₽).
    assert_eq!(overview.expenses_delta_percent, Some(-4));
    assert_eq!(overview.summary.savings_rate_bp, Some(1_304));
    let over = overview
        .limits
        .iter()
        .filter(|r| r.level == Some(crate::dto::LimitLevelDto::Over))
        .count();
    assert_eq!(usize::try_from(overview.over_count).unwrap(), over);
    let shares = &overview.summary.by_status.share_bp;
    assert!(shares.paid > 0 && shares.planned > 0);
    assert_eq!(
        overview.summary.by_status.total,
        overview.summary.by_status.paid
            + overview.summary.by_status.debt
            + overview.summary.by_status.unplanned
            + overview.summary.by_status.planned
    );

    // До сентября в году меньше двух месяцев с данными — сравнивать не с чем.
    let january = summary::month(
        &db,
        "2026-01",
        NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
    )
    .unwrap();
    assert_eq!(january.expenses_delta_percent, None);
}

#[test]
fn month_series_covers_year_last_twelve_and_only_months_with_data() {
    use crate::dto::SeriesRangeDto::{All, Last12, Year};
    let (_dir, db) = seeded();
    let months = |range| {
        summary::series(&db, "2026-09", range)
            .unwrap()
            .into_iter()
            .map(|m| m.month)
            .collect::<Vec<_>>()
    };
    let year = months(Year);
    assert_eq!(
        (year.first().map(String::as_str), year.len()),
        (Some("2026-01"), 12)
    );
    let last12 = months(Last12);
    assert_eq!(
        (
            last12.first().map(String::as_str),
            last12.last().map(String::as_str)
        ),
        (Some("2025-10"), Some("2026-09"))
    );
    assert_eq!(last12.len(), 12);
    // Данные есть только с января по сентябрь 2026: пустые месяцы в «Всё» не попадают.
    let all = months(All);
    assert_eq!(all.len(), 9);
    assert_eq!(all.first().map(String::as_str), Some("2026-01"));
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
    let before = summary::month(&db, "2026-09", today())
        .unwrap()
        .summary
        .expenses;
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
        summary::month(&db, "2026-09", today())
            .unwrap()
            .summary
            .expenses,
        before + rub(1000)
    );

    let updated =
        records::tx_set_status(&mut db, TxId(created.id), TxStatusDto::Planned, now()).unwrap();
    assert_eq!(updated.previous.status, TxStatusDto::Paid);

    records::tx_delete(&mut db, TxId(created.id), now()).unwrap();
    assert_eq!(
        summary::month(&db, "2026-09", today())
            .unwrap()
            .summary
            .expenses,
        before
    );
    records::tx_restore(&mut db, TxId(created.id), now()).unwrap();
    assert_eq!(
        summary::month(&db, "2026-09", today())
            .unwrap()
            .summary
            .expenses,
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
}

#[test]
fn savings_overview_matches_the_bonds_golden_for_the_seeded_accumulation() {
    let (_dir, db) = seeded();
    let dto = savings::overview(&db, 2026, today()).unwrap();
    assert_eq!(dto.items.len(), 1);
    let item = &dto.items[0];
    assert_eq!(item.months.len(), 12);
    // golden: баланс на конец сентября 153 566,17, на конец декабря 158 652,00
    let september = item.months.iter().find(|m| m.month == "2026-09").unwrap();
    assert!((september.balance - 15_356_617).abs() <= 100);
    assert!((item.balance - 15_865_200).abs() <= 100);
    assert_eq!(dto.total_balance, item.balance);
    assert_eq!(item.effective_rate_bp, 1392);
    assert_eq!(item.params.annual_rate_bp, 1600);
    assert_eq!(item.plan_kind, crate::dto::SavingsPlanKindDto::Percent);
    assert_eq!(item.plan_rate_bp, 1400);
    // golden: B на горизонте 5 лет — 2 414 468,03
    let b = item
        .scenarios
        .iter()
        .find(|s| s.scenario == crate::dto::ForecastKindDto::B)
        .unwrap();
    assert!((b.y5 - 241_446_803).abs() <= 100);
    assert_eq!(b.balances.len(), 60);
    assert_eq!(b.balances.last().copied(), Some(b.y5));
    assert_eq!(dto.total_scenarios.len(), 3);
}

#[test]
fn savings_charts_come_ready_from_the_backend() {
    let (_dir, db) = seeded();
    let dto = savings::overview(&db, 2026, today()).unwrap();
    let item = &dto.items[0];
    let fact = &item.fact_chart;
    assert_eq!(fact.categories.len(), item.months.len());
    assert_eq!(fact.categories[0], "Янв");
    assert_eq!(fact.series.len(), 2);
    assert_eq!(fact.series[0].name, "Баланс");
    // Ось в рублях: копейки переведены на бэкенде.
    let dec = fact.series[0].values.last().copied().flatten().unwrap();
    assert!((dec - item.balance as f64 / 100.0).abs() < 0.01);
    let forecast = &item.forecast_chart;
    assert_eq!(forecast.categories.len(), 60);
    assert_eq!(forecast.categories[0], "Янв 2027");
    assert_eq!(forecast.series.len(), 3);
    assert_eq!(dto.total_forecast_chart.series.len(), 3);
}

#[test]
fn savings_params_and_fixed_plan_are_saved_and_shown() {
    let (_dir, mut db) = seeded();
    let id = category(&db, "Сбережения");
    savings::params_set(
        &mut db,
        id.0,
        &crate::dto::SavingsParamsDto {
            annual_rate_bp: 900,
            tax_bp: 0,
            initial_balance: rub(10_000),
            initial_month: "2026-01".to_owned(),
        },
    )
    .unwrap();
    savings::fixed_set(&mut db, id.0, "2026-09", rub(5_000)).unwrap();
    let dto = savings::overview(&db, 2026, today()).unwrap();
    let item = &dto.items[0];
    assert_eq!(
        (item.params.annual_rate_bp, item.params.initial_balance),
        (900, rub(10_000))
    );
    assert_eq!(item.plan_kind, crate::dto::SavingsPlanKindDto::Fixed);
    assert_eq!(item.plan_fixed, Some(rub(5_000)));
    assert_eq!(item.plan, rub(5_000));
    assert_eq!(dto.month_plan, rub(5_000));

    let bad = savings::params_set(
        &mut db,
        id.0,
        &crate::dto::SavingsParamsDto {
            annual_rate_bp: 20_000,
            tax_bp: 0,
            initial_balance: 0,
            initial_month: "2026-01".to_owned(),
        },
    );
    assert!(bad.is_err(), "ставка вне 0–100 % отклоняется");
    let food = category(&db, "Продукты");
    assert!(
        savings::fixed_set(&mut db, food.0, "2026-09", rub(1)).is_err(),
        "только накопления"
    );
}

#[test]
fn month_plan_numbers_are_consistent_and_follow_the_formula() {
    let (_dir, db) = seeded();
    let p = plan::month(&db, "2026-09").unwrap();
    assert!(!p.locked);
    assert_eq!(p.income, rub(163_000));
    assert_eq!(
        p.unallocated,
        p.income - p.plan_expenses - p.plan_savings - p.plan_repayments
    );
    assert_eq!(p.plan_repayments, 0);
    assert!(p.plan_expenses > 0 && p.plan_savings > 0);
    let expected = if p.unallocated > 0 {
        PlanBalanceDto::Unallocated
    } else if p.unallocated < 0 {
        PlanBalanceDto::Over
    } else {
        PlanBalanceDto::Balanced
    };
    assert_eq!(p.balance, expected);
    assert_eq!(
        p.unplanned,
        summary::month(&db, "2026-09", today())
            .unwrap()
            .summary
            .by_status
            .unplanned
    );
    assert!(summary::month(&db, "2026-09", today()).is_ok_and(|o| !o.plan_locked));
}

#[test]
fn empty_month_plan_is_reported_as_empty() {
    let (_dir, db) = seeded();
    let p = plan::month(&db, "2027-05").unwrap();
    assert_eq!(p.balance, PlanBalanceDto::Empty);
    assert_eq!((p.income, p.plan_expenses, p.unallocated), (0, 0, 0));
}

#[test]
fn lock_freezes_the_plan_against_fact_edits_and_unlock_restores_it() {
    let (_dir, mut db) = seeded();
    let before = plan::month(&db, "2026-09").unwrap();
    let locked = plan::lock(&mut db, "2026-09", now()).unwrap();
    assert!(locked.locked);
    assert_eq!(locked.plan_expenses, before.plan_expenses);
    assert_eq!(locked.unallocated, before.unallocated);
    assert!(summary::month(&db, "2026-09", today()).unwrap().plan_locked);

    // Оплаченная трата выросла на 1 000 ₽: факт меняется, план — нет.
    let row = db
        .transactions_of_month(planning_budget_core::YearMonth::parse("2026-09").unwrap())
        .unwrap()
        .into_iter()
        .find(|t| t.status == planning_budget_core::TxStatus::Paid)
        .unwrap();
    db.transaction_update(
        row.id,
        &planning_budget_storage::TransactionPatch {
            amount: Some(planning_budget_core::Money::from_kopecks(
                row.amount.kopecks() + rub(1_000),
            )),
            ..planning_budget_storage::TransactionPatch::default()
        },
        now(),
    )
    .unwrap();
    let after = plan::month(&db, "2026-09").unwrap();
    assert_eq!(after.plan_expenses, before.plan_expenses);
    let moved = after
        .rows
        .iter()
        .find(|r| r.category_id == row.category_id.0)
        .unwrap();
    assert_eq!(moved.deviation, moved.fact - moved.plan);
    assert!(moved.deviation >= rub(1_000));

    let unlocked = plan::unlock(&mut db, "2026-09").unwrap();
    assert!(!unlocked.locked);
    assert_eq!(unlocked.plan_expenses, before.plan_expenses + rub(1_000));
}

#[test]
fn lock_and_unlock_errors_carry_ui_keys() {
    let (_dir, mut db) = seeded();
    let err = plan::unlock(&mut db, "2026-09").unwrap_err();
    assert!(
        matches!(&err, AppError::Conflict { message_key } if message_key == "errors.plan.not_locked")
    );
    plan::lock(&mut db, "2026-09", now()).unwrap();
    let err = plan::lock(&mut db, "2026-09", now()).unwrap_err();
    assert!(
        matches!(&err, AppError::Conflict { message_key } if message_key == "errors.plan.already_locked")
    );
}

#[test]
fn copy_from_previous_fills_the_next_month_once() {
    let (_dir, mut db) = seeded();
    let before = plan::month(&db, "2026-09").unwrap();
    let copied = plan::copy_from_previous(&mut db, "2026-10", now()).unwrap();
    assert!(copied.plan_expenses > 0);
    // Скопирован ровно план сентября: плановые, оплаченные и долговые строки.
    assert_eq!(copied.plan_expenses, before.plan_expenses);
    let err = plan::copy_from_previous(&mut db, "2026-10", now()).unwrap_err();
    assert!(
        matches!(&err, AppError::Conflict { message_key } if message_key == "errors.plan.not_empty")
    );
}

fn debt_input(db: &Db, schedule: Vec<(&str, i64)>) -> crate::dto::DebtInput {
    crate::dto::DebtInput {
        lender: "Кредитная карта".to_owned(),
        amount: rub(30_000),
        taken_month: "2026-09".to_owned(),
        taken_date: None,
        category_id: Some(category(db, "Одежда").0),
        comment: None,
        schedule: schedule
            .into_iter()
            .map(|(month, amount)| crate::dto::SchedulePaymentDto {
                month: month.to_owned(),
                amount: rub(amount),
            })
            .collect(),
    }
}

fn three_payments() -> Vec<(&'static str, i64)> {
    vec![
        ("2026-10", 10_000),
        ("2026-11", 10_000),
        ("2026-12", 10_000),
    ]
}

#[test]
fn debt_overview_reports_remaining_share_and_month_payments() {
    use super::debts;
    let (_dir, mut db) = seeded();
    let created = {
        let input = debt_input(&db, three_payments());
        debts::create(&mut db, input, now())
    }
    .unwrap();
    assert_eq!(
        (created.amount, created.remaining, created.paid_bp),
        (rub(30_000), rub(30_000), 0)
    );
    assert_eq!(
        created.next_payment.as_ref().map(|p| p.month.as_str()),
        Some("2026-10")
    );

    let paid = debts::set_payment_status(
        &mut db,
        created.payments[0].id,
        crate::dto::DebtPaymentStatusDto::Paid,
        Some("2026-10-05"),
        now(),
    )
    .unwrap();
    assert_eq!((paid.remaining, paid.paid_bp), (rub(20_000), 3_333));
    assert_eq!(paid.payments[0].paid_date.as_deref(), Some("2026-10-05"));

    let october = debts::overview(&db, "2026-10", false).unwrap();
    assert_eq!((october.open_count, october.closed_count), (1, 0));
    assert_eq!(october.remaining_total, rub(20_000));
    assert_eq!(
        (october.payments_planned, october.payments_paid),
        (rub(10_000), rub(10_000))
    );
    let november = debts::overview(&db, "2026-11", false).unwrap();
    assert_eq!(
        (november.payments_planned, november.payments_paid),
        (rub(10_000), 0)
    );
}

#[test]
fn paying_everything_closes_the_debt_and_hides_it_by_default() {
    use super::debts;
    let (_dir, mut db) = seeded();
    let created = {
        let input = debt_input(&db, three_payments());
        debts::create(&mut db, input, now())
    }
    .unwrap();
    for payment in &created.payments {
        debts::set_payment_status(
            &mut db,
            payment.id,
            crate::dto::DebtPaymentStatusDto::Paid,
            None,
            now(),
        )
        .unwrap();
    }
    let hidden = debts::overview(&db, "2026-10", false).unwrap();
    assert!(hidden.debts.is_empty());
    assert_eq!((hidden.open_count, hidden.closed_count), (0, 1));
    let shown = debts::overview(&db, "2026-10", true).unwrap();
    assert!(shown.debts[0].closed && shown.debts[0].paid_bp == 10_000);
    assert!(shown.debts[0].next_payment.is_none());
}

#[test]
fn schedule_preview_builds_equal_parts_and_single_payment() {
    use super::debts;
    use crate::dto::DebtScheduleKindDto::{EqualParts, Single};
    let equal =
        debts::schedule_preview(rub(10_000) + 1, "2026-09", EqualParts { months: 3 }).unwrap();
    let months: Vec<&str> = equal.iter().map(|r| r.month.as_str()).collect();
    assert_eq!(months, vec!["2026-10", "2026-11", "2026-12"]);
    assert_eq!(equal.iter().map(|r| r.amount).sum::<i64>(), rub(10_000) + 1);
    assert!(
        equal[2].amount >= equal[0].amount,
        "остаток копеек в последнем платеже"
    );

    let single = debts::schedule_preview(
        rub(500),
        "2026-09",
        Single {
            month: "2026-12".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(single.len(), 1);

    let early = debts::schedule_preview(
        rub(500),
        "2026-09",
        Single {
            month: "2026-08".to_owned(),
        },
    );
    assert!(
        matches!(early, Err(AppError::Validation { message_key, .. }) if message_key == "errors.debt.schedule")
    );
    assert!(debts::schedule_preview(rub(500), "2026-09", EqualParts { months: 0 }).is_err());
}

#[test]
fn bad_schedule_is_rejected_with_a_ui_key_and_nothing_is_saved() {
    use super::debts;
    let (_dir, mut db) = seeded();
    let short = {
        let input = debt_input(&db, vec![("2026-10", 10_000)]);
        debts::create(&mut db, input, now())
    };
    assert!(
        matches!(short, Err(AppError::Validation { message_key, .. }) if message_key == "errors.debt.schedule")
    );
    assert!(
        debts::overview(&db, "2026-10", true)
            .unwrap()
            .debts
            .is_empty()
    );
}

#[test]
fn debt_from_a_debt_status_transaction_takes_its_amount_and_category() {
    use super::debts;
    let (_dir, mut db) = seeded();
    let clothes = category(&db, "Одежда").0;
    let created = records::tx_create(
        &mut db,
        TransactionInput {
            month: "2026-09".to_owned(),
            date: None,
            category_id: clothes,
            title: "Куртка".to_owned(),
            amount: rub(30_000),
            status: TxStatusDto::Debt,
            comment: None,
        },
        now(),
    )
    .unwrap();
    let rows = debts::schedule_preview(
        rub(30_000),
        "2026-09",
        crate::dto::DebtScheduleKindDto::EqualParts { months: 3 },
    )
    .unwrap();
    let debt = debts::create_from_transaction(&mut db, TxId(created.id), "Рассрочка", &rows, now())
        .unwrap();
    assert_eq!(debt.amount, rub(30_000));
    assert_eq!(debt.taken_month, "2026-09");
    assert_eq!(debt.transaction_id, Some(created.id));
    assert_eq!(debt.category_id, Some(clothes));
}

#[test]
fn repayments_appear_in_the_month_plan_and_reduce_unallocated() {
    use super::debts;
    let (_dir, mut db) = seeded();
    let before = plan::month(&db, "2026-10").unwrap();
    let created = {
        let input = debt_input(&db, three_payments());
        debts::create(&mut db, input, now())
    }
    .unwrap();
    let after = plan::month(&db, "2026-10").unwrap();
    assert_eq!(after.plan_repayments, rub(10_000));
    assert_eq!(after.unallocated, before.unallocated - rub(10_000));
    assert_eq!(after.repayments.len(), 1);
    assert_eq!(after.repayments[0].lender, "Кредитная карта");
    assert_eq!(after.repayments[0].payment_id, created.payments[0].id);
    // Погашение не расход: расходы месяца не выросли.
    assert_eq!(after.plan_expenses, before.plan_expenses);
}

#[test]
fn delete_and_restore_round_trip_the_debt() {
    use super::debts;
    let (_dir, mut db) = seeded();
    let created = {
        let input = debt_input(&db, three_payments());
        debts::create(&mut db, input, now())
    }
    .unwrap();
    let removed = debts::delete(&mut db, DebtId(created.id), now()).unwrap();
    assert_eq!(removed.id, created.id);
    assert!(
        debts::overview(&db, "2026-10", true)
            .unwrap()
            .debts
            .is_empty()
    );
    assert_eq!(plan::month(&db, "2026-10").unwrap().plan_repayments, 0);
    let restored = debts::restore(&mut db, DebtId(created.id), now()).unwrap();
    assert_eq!(restored.payments.len(), 3);
}

fn wizard_dto(db: &Db) -> crate::dto::PlanWizardInputDto {
    use crate::dto::{
        PlanWizardInputDto, WizardIncomeDto, WizardLineDto, WizardSavingsDto, WizardSavingsPlanDto,
    };
    PlanWizardInputDto {
        incomes: vec![WizardIncomeDto {
            source_name: "Зарплата".to_owned(),
            amount: rub(100_000),
        }],
        lines: vec![
            WizardLineDto {
                category_id: category(db, "Продукты").0,
                amount: rub(30_000),
            },
            WizardLineDto {
                category_id: category(db, "Кафе и доставка").0,
                amount: rub(10_000),
            },
        ],
        savings: vec![WizardSavingsDto {
            category_id: category(db, "Сбережения").0,
            plan: WizardSavingsPlanDto::Percent { rate_bp: 1000 },
        }],
    }
}

#[test]
fn wizard_preview_matches_what_apply_produces_and_writes_nothing() {
    let (_dir, mut db) = seeded();
    let input = wizard_dto(&db);
    let preview = plan::preview(&db, "2026-11", &input).unwrap();
    assert_eq!(preview.income, rub(100_000));
    assert_eq!(preview.plan_expenses, rub(40_000));
    assert_eq!(preview.plan_savings, rub(10_000));
    assert_eq!(preview.unallocated, rub(50_000));
    assert_eq!(preview.balance, PlanBalanceDto::Unallocated);
    assert!(!preview.locked);
    // Предпросмотр ничего не записал.
    assert_eq!(plan::month(&db, "2026-11").unwrap().income, 0);

    let applied = plan::wizard_apply(&mut db, "2026-11", &input, now()).unwrap();
    assert!(applied.locked);
    assert_eq!(
        (
            applied.income,
            applied.plan_expenses,
            applied.plan_savings,
            applied.unallocated
        ),
        (
            preview.income,
            preview.plan_expenses,
            preview.plan_savings,
            preview.unallocated
        )
    );
}

#[test]
fn wizard_preview_skips_incomplete_rows_and_reports_over_plan() {
    let (_dir, db) = seeded();
    let mut input = wizard_dto(&db);
    input.lines[0].amount = 0;
    input.lines.push(crate::dto::WizardLineDto {
        category_id: 9_999,
        amount: rub(5_000),
    });
    input.lines.push(crate::dto::WizardLineDto {
        category_id: category(&db, "Одежда").0,
        amount: rub(200_000),
    });
    let preview = plan::preview(&db, "2026-11", &input).unwrap();
    assert_eq!(preview.plan_expenses, rub(10_000) + rub(200_000));
    assert_eq!(preview.balance, PlanBalanceDto::Over);
    assert!(preview.unallocated < 0);
}

#[test]
fn wizard_apply_errors_carry_ui_keys_and_keep_the_month_untouched() {
    let (_dir, mut db) = seeded();
    let mut bad = wizard_dto(&db);
    bad.lines.push(crate::dto::WizardLineDto {
        category_id: category(&db, "Сбережения").0,
        amount: rub(1_000),
    });
    let err = plan::wizard_apply(&mut db, "2026-11", &bad, now()).unwrap_err();
    assert!(
        matches!(&err, AppError::Validation { message_key, .. } if message_key == "errors.plan.line_is_savings")
    );
    assert_eq!(plan::month(&db, "2026-11").unwrap().income, 0);

    let good = wizard_dto(&db);
    plan::wizard_apply(&mut db, "2026-11", &good, now()).unwrap();
    let err = plan::wizard_apply(&mut db, "2026-11", &good, now()).unwrap_err();
    assert!(
        matches!(&err, AppError::Conflict { message_key } if message_key == "errors.plan.locked")
    );
}

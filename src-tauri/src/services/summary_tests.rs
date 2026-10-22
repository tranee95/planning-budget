//! Сводки и обзор месяца: сервисы данных без Tauri.

#![allow(clippy::unwrap_used, reason = "тесты")]

use super::test_support::*;
use super::{plan, records, savings, summary};
use crate::dto::{TransactionInput, TxStatusDto};
use chrono::NaiveDate;
use planning_budget_core::TxId;

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
fn saved_on_overview_matches_deposited_on_savings_for_every_seed_month() {
    use planning_budget_core::{CategoryKind, TxStatus, YearMonth};
    let (_dir, db) = seeded();
    for month in 1..=9 {
        let key = format!("2026-{month:02}");
        let ym = YearMonth::parse(&key).unwrap();
        let saved = plan::month(&db, &key).unwrap().saved;
        let progress = savings::progress(&db, &key).unwrap();
        let deposited = progress
            .months
            .iter()
            .find(|m| m.month == key)
            .map_or(0, |m| m.deposited);
        // Третье независимое определение: оплаченные записи категорий-накоплений месяца.
        let data = db.dataset(ym, ym).unwrap();
        let paid_by_hand: i64 = data
            .transactions
            .iter()
            .filter(|t| t.month == ym && t.status != TxStatus::Planned)
            .filter(|t| {
                data.categories
                    .iter()
                    .any(|c| c.id == t.category_id && c.kind == CategoryKind::Savings)
            })
            .map(|t| t.amount.kopecks())
            .sum();
        assert_eq!(saved, deposited, "{key}: Обзор и Сбережения");
        assert_eq!(saved, paid_by_hand, "{key}: сумма записей");
    }
    // Сентябрь seed: всё накопление в плане. «Отложено» 0, план — отдельная величина.
    let sep = plan::month(&db, "2026-09").unwrap();
    assert_eq!(sep.saved, 0);
    assert!(sep.plan_savings > 0);
}

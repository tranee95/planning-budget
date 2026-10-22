//! Накопления: сервисы данных без Tauri.

#![allow(clippy::unwrap_used, reason = "тесты")]

use super::test_support::*;
use super::{categories, records, savings, summary};
use crate::dto::{CategoryInput, CategoryKindDto, TransactionInput, TxStatusDto};
use planning_budget_core::{CategoryId, TxId};
use planning_budget_storage::Db;
use tempfile::TempDir;

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
fn fixed_savings_plan_is_saved_and_shown_in_the_month_plan() {
    let (_dir, mut db) = seeded();
    let id = category(&db, "Сбережения");
    savings::fixed_set(&mut db, id.0, "2026-09", rub(5_000)).unwrap();
    let plan = super::plan::month(&db, "2026-09").unwrap();
    let fixed = plan
        .savings_fixed
        .iter()
        .find(|s| s.category_id == id.0)
        .unwrap();
    assert_eq!(fixed.fixed, rub(5_000));
    let row = plan.rows.iter().find(|r| r.category_id == id.0).unwrap();
    assert_eq!(row.plan, rub(5_000));
    assert_eq!(plan.plan_savings, rub(5_000));
    // До месяца начала действует прежний процент, фиксированной суммы нет.
    assert!(
        super::plan::month(&db, "2026-08")
            .unwrap()
            .savings_fixed
            .is_empty()
    );
    let food = category(&db, "Продукты");
    assert!(
        savings::fixed_set(&mut db, food.0, "2026-09", rub(1)).is_err(),
        "только накопления"
    );
}

/// Сумма накопления за месяц (Q-17): обычная оплаченная трата в категории-сбережении.
fn put_saving(db: &mut Db, category_id: i64, month: &str, amount: i64) -> i64 {
    records::tx_create(
        db,
        TransactionInput {
            month: month.into(),
            date: None,
            category_id,
            title: "Накопление".into(),
            amount,
            status: TxStatusDto::Paid,
            comment: None,
        },
        now(),
    )
    .unwrap()
    .id
}

#[test]
fn savings_progress_sums_amounts_over_the_whole_period() {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    let savings = category(&db, "Сбережения").0;
    let empty = savings::progress(&db, "2026-10").unwrap();
    assert_eq!((empty.total, empty.months.len()), (0, 0));
    put_saving(&mut db, savings, "2026-08", rub(10_000));
    put_saving(&mut db, savings, "2026-09", rub(10_000));
    // Октябрь: 12 000 ₽ тремя суммами по 4 000 ₽ — одна запись месяца в графике.
    for _ in 0..3 {
        put_saving(&mut db, savings, "2026-10", rub(4_000));
    }
    let p = savings::progress(&db, "2026-10").unwrap();
    assert_eq!(p.total, rub(32_000));
    assert_eq!(p.months.len(), 3);
    assert_eq!(p.months[2].deposited, rub(12_000));
    assert_eq!(p.chart.categories.len(), 3);
    assert!(savings::progress(&db, "2026-13").is_err());
}

#[test]
fn savings_progress_lists_every_accumulation_including_a_closed_goal() {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    let cushion = category(&db, "Сбережения").0;
    let trip = categories::create(
        &mut db,
        crate::dto::CategoryInput {
            name: "Отпуск".into(),
            kind: crate::dto::CategoryKindDto::Savings,
            color: "#D9893A".into(),
            note: None,
        },
        now(),
    )
    .unwrap()
    .id;
    put_saving(&mut db, cushion, "2026-10", rub(5_000));
    put_saving(&mut db, trip, "2026-09", rub(20_000));
    put_saving(&mut db, trip, "2026-09", rub(10_000));
    // Цель «Отпуск» достигнута и закрыта (архив): её суммы остаются в графике и в итоге.
    categories::archive(&mut db, CategoryId(trip), today(), now()).unwrap();
    let p = savings::progress(&db, "2026-10").unwrap();
    assert_eq!(p.total, rub(35_000));
    assert_eq!(p.categories.len(), 2);
    let closed = p.categories.iter().find(|c| c.name == "Отпуск").unwrap();
    assert!(closed.archived);
    assert_eq!(closed.total, rub(30_000));
    assert_eq!(closed.color, "#D9893A");
    assert_eq!(p.chart.series.len(), 2);
    let series = p.chart.series.iter().find(|s| s.name == "Отпуск").unwrap();
    assert_eq!(series.color, format!("category:{trip}"));
    assert_eq!(series.values, vec![Some(30_000.0), Some(0.0)]);
}

#[test]
fn savings_amounts_lower_the_free_balance_and_deleting_one_restores_it() {
    let (_dir, mut db) = seeded();
    let savings = category(&db, "Сбережения").0;
    // Месяц без записей: итоги до и после сравниваются как есть.
    let before = summary::month(&db, "2026-11", today()).unwrap().summary;
    let first = put_saving(&mut db, savings, "2026-11", rub(7_000));
    put_saving(&mut db, savings, "2026-11", rub(3_000));
    let after = summary::month(&db, "2026-11", today()).unwrap().summary;
    // Суммы — расход бюджета: уменьшают свободный остаток и входят в сбережения; «Расходы» без сбережений.
    assert_eq!(after.expenses, before.expenses);
    assert_eq!(after.savings, before.savings + rub(10_000));
    assert_eq!(after.free, before.free - rub(10_000));
    records::tx_delete(&mut db, TxId(first), now()).unwrap();
    let rest = summary::month(&db, "2026-11", today()).unwrap().summary;
    assert_eq!(rest.savings, before.savings + rub(3_000));
    assert_eq!(rest.free, before.free - rub(3_000));
}

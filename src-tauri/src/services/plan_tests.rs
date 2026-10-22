//! План месяца, мастер плана, автоперенос: сервисы данных без Tauri.

#![allow(clippy::unwrap_used, reason = "тесты")]

use super::test_support::*;
use super::{plan, records, summary};
use crate::AppError;
use crate::clock::Now;
use crate::dto::{PlanBalanceDto, TxStatusDto};
use chrono::{NaiveDate, TimeZone, Utc};
use planning_budget_storage::Db;

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

#[test]
fn carry_forward_fills_a_new_month_once_and_rejects_a_bad_month() {
    let (_dir, mut db) = seeded();
    let before = plan::month(&db, "2026-09").unwrap();
    let late = Now {
        utc: Utc.with_ymd_and_hms(2026, 10, 2, 9, 0, 0).unwrap(),
        today: NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
    };
    let created = plan::carry_forward(&mut db, "2026-10", &late).unwrap();
    assert!(created > 0);
    assert_eq!(
        plan::month(&db, "2026-10").unwrap().plan_expenses,
        before.plan_expenses
    );
    assert_eq!(plan::carry_forward(&mut db, "2026-10", &late).unwrap(), 0);
    assert!(plan::carry_forward(&mut db, "2026-13", &late).is_err());
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

/// UX-4: после мастера все строки в статусе «План», «Фактически» равно нулю; оплата переносит сумму.
#[test]
fn plan_actual_is_zero_after_the_wizard_and_follows_payments() {
    let (_dir, mut db) = seeded();
    let input = wizard_dto(&db);
    let applied = plan::wizard_apply(&mut db, "2026-11", &input, now()).unwrap();
    let food_id = category(&db, "Продукты").0;
    let food = applied
        .rows
        .iter()
        .find(|r| r.category_id == food_id)
        .unwrap();
    assert_eq!(
        (food.plan, food.fact, food.actual, food.deviation),
        (rub(30_000), rub(30_000), 0, 0)
    );
    assert!(applied.rows.iter().all(|r| r.actual == 0));

    let line = db
        .transactions_of_month(planning_budget_core::YearMonth::parse("2026-11").unwrap())
        .unwrap()
        .into_iter()
        .find(|t| t.category_id.0 == food_id)
        .unwrap();
    records::tx_set_status(&mut db, line.id, TxStatusDto::Paid, now()).unwrap();
    let paid = plan::month(&db, "2026-11").unwrap();
    let food = paid.rows.iter().find(|r| r.category_id == food_id).unwrap();
    assert_eq!((food.fact, food.actual), (rub(30_000), rub(30_000)));
    let cafe = paid
        .rows
        .iter()
        .find(|r| r.category_id == category(&db, "Кафе и доставка").0)
        .unwrap();
    assert_eq!(cafe.actual, 0);
}
#[test]
fn overview_exposes_free_actual_and_remaining_weeks_only_for_the_current_month() {
    let (_dir, db) = seeded();
    // «Сегодня» 15 сентября: до конца месяца 16 дней. Значения посчитаны независимо от кода:
    // доход 163 000 ₽ − оплаченное 88 828 ₽ = 74 172 ₽; 4 272 349 коп. × 7 / 16 → 1 869 153 коп.
    let sep = summary::month(&db, "2026-09", today()).unwrap().summary;
    assert_eq!((sep.free, sep.free_actual), (4_272_349, 7_417_200));
    assert_eq!(sep.per_week, 1_869_153);
    assert!((sep.weeks_left.unwrap() - 16.0 / 7.0).abs() < 1e-9);
    // Прошлый месяц: прежнее правило, остаток ÷ 4, без «оставшихся недель».
    let aug = summary::month(&db, "2026-08", today()).unwrap().summary;
    assert_eq!(aug.per_week, 751_934);
    assert_eq!(aug.weeks_left, None);
    assert_eq!(aug.free_actual, aug.free);
}

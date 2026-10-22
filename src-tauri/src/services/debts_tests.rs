//! Долги: сервисы данных без Tauri.

#![allow(clippy::unwrap_used, reason = "тесты")]

use super::test_support::*;
use super::{plan, records};
use crate::AppError;
use crate::dto::{TransactionInput, TxStatusDto};
use planning_budget_core::{DebtId, TxId};

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
    assert_eq!(equal.total, rub(10_000) + 1);
    let equal = equal.rows;
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
    assert_eq!(single.rows.len(), 1);
    assert_eq!(single.total, rub(500));

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
    let debt =
        debts::create_from_transaction(&mut db, TxId(created.id), "Рассрочка", &rows.rows, now())
            .unwrap();
    assert_eq!(debt.amount, rub(30_000));
    assert_eq!(debt.taken_month, "2026-09");
    assert_eq!(debt.transaction_id, Some(created.id));
    assert_eq!(debt.category_id, Some(clothes));
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

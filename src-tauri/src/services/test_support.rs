//! Общие помощники тестов сервисов данных: сид → база.

#![allow(clippy::unwrap_used, reason = "тесты")]

use super::devseed;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use planning_budget_core::CategoryId;
use planning_budget_storage::Db;
use tempfile::TempDir;

pub(super) const KEY: [u8; 32] = [7; 32];

pub(super) fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap()
}

pub(super) fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
}

pub(super) fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    devseed::load(&mut db, now()).unwrap();
    (dir, db)
}

pub(super) fn category(db: &Db, name: &str) -> CategoryId {
    db.category_id_by_name(name).unwrap().unwrap()
}

pub(super) fn rub(kopecks: i64) -> i64 {
    kopecks * 100
}

pub(super) fn debt_input(db: &Db, schedule: Vec<(&str, i64)>) -> crate::dto::DebtInput {
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

pub(super) fn three_payments() -> Vec<(&'static str, i64)> {
    vec![
        ("2026-10", 10_000),
        ("2026-11", 10_000),
        ("2026-12", 10_000),
    ]
}

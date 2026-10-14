//! «Свободный остаток по факту», «На неделю» по оставшимся неделям
//! и согласованность «Отложено» на «Обзоре» и «Сбережениях».
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::calc::Ledger;
use planning_budget_core::{
    BasisPoints, CalendarDay, Category, CategoryId, CategoryKind, DataSet, Debt, DebtId,
    DebtPayment, DebtPaymentStatus, Income, IncomeId, IncomeStatus, Money, Settings, Transaction,
    TxId, TxStatus, YearMonth, load_seed,
};

const SEED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/seed-2026.json"
));

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn day(month: &str, d: u8) -> CalendarDay {
    CalendarDay::new(ym(month), d).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

const FOOD: CategoryId = CategoryId(1);
const SAVE: CategoryId = CategoryId(2);

/// Январь 2026: доход 100 000 ₽; расходы 5 000 ₽ оплачено и 30 000 ₽ в плане; накопления
/// 6 000 ₽ оплачено и 14 000 ₽ в плане; заём 10 000 ₽; погашения 4 000 ₽ оплачено и 3 000 ₽ в плане.
fn january() -> DataSet {
    let category = |id, name: &str, kind| Category {
        id,
        name: name.to_owned(),
        kind,
        color: "#000000".to_owned(),
        sort_order: 0,
        note: None,
        archived: false,
    };
    let tx = |id, category_id, amount, status| Transaction {
        id: TxId(id),
        month: ym("2026-01"),
        category_id,
        title: "Запись".to_owned(),
        amount,
        status,
        planned_amount: None,
    };
    DataSet {
        settings: Settings {
            savings_min: BasisPoints(1300),
            savings_norm: BasisPoints(1400),
            savings_max: BasisPoints(1500),
            weeks_per_month: 4,
        },
        categories: vec![
            category(FOOD, "Продукты", CategoryKind::Mandatory),
            category(SAVE, "Сбережения", CategoryKind::Savings),
        ],
        limits: Vec::new(),
        savings_rates: Vec::new(),
        savings_overrides: BTreeMap::new(),
        transactions: vec![
            tx(1, FOOD, rub(5_000), TxStatus::Paid),
            tx(2, FOOD, rub(30_000), TxStatus::Planned),
            tx(3, SAVE, rub(6_000), TxStatus::Paid),
            tx(4, SAVE, rub(14_000), TxStatus::Planned),
        ],
        incomes: vec![Income {
            id: IncomeId(1),
            month: ym("2026-01"),
            source_name: "Зарплата".to_owned(),
            amount: rub(100_000),
            status: IncomeStatus::Received,
        }],
        debts: vec![Debt {
            id: DebtId(1),
            lender: "Кредитная карта".to_owned(),
            amount: rub(10_000),
            taken_month: ym("2026-01"),
            category_id: None,
        }],
        debt_payments: vec![
            DebtPayment {
                debt_id: DebtId(1),
                month: ym("2026-01"),
                amount: rub(4_000),
                status: DebtPaymentStatus::Paid,
            },
            DebtPayment {
                debt_id: DebtId(1),
                month: ym("2026-01"),
                amount: rub(3_000),
                status: DebtPaymentStatus::Planned,
            },
        ],
        locked_plans: BTreeMap::new(),
        savings_params: BTreeMap::new(),
    }
}

#[test]
fn free_actual_drops_planned_lines_and_keeps_loans_and_repayments() {
    let data = january();
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-01"))
        .unwrap();
    // С планом: 100 − 35 − 20 + 10 − 4 = 51 тыс. ₽ (формула §1 не менялась).
    assert_eq!(s.free, rub(51_000));
    // По факту: 100 − 5 − 6 + 10 − 4 = 95 тыс. ₽; плановые 30 000 и 14 000 не вычитаются.
    assert_eq!(s.free_actual, rub(95_000));
}

#[test]
fn free_actual_equals_free_when_nothing_is_planned() {
    let mut data = january();
    data.transactions.retain(|t| t.status != TxStatus::Planned);
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-01"))
        .unwrap();
    assert_eq!(s.free_actual, s.free);
    assert_eq!(s.free_actual, rub(95_000));
}

#[test]
fn free_actual_of_a_month_with_only_planned_lines_is_the_income() {
    let mut data = january();
    data.transactions.retain(|t| t.status == TxStatus::Planned);
    data.debts.clear();
    data.debt_payments.clear();
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary(ym("2026-01"))
        .unwrap();
    assert_eq!(s.free, rub(100_000 - 44_000));
    assert_eq!(s.free_actual, rub(100_000));
}

#[test]
fn free_actual_on_the_seed_matches_independent_values() {
    // Значения посчитаны отдельным скриптом по seed-2026.json: доход − все траты, кроме
    // записей со статусом «План». Январь–август плановых записей нет, в сентябре
    // 10 199 ₽ расходов и 21 249,51 ₽ накоплений в плане: 163 000 − 88 828 = 74 172.
    let data = load_seed(SEED).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    let expected = [
        ("2026-01", 2_296_592),
        ("2026-02", 1_711_824),
        ("2026-03", 2_089_678),
        ("2026-04", 3_929_487),
        ("2026-05", 3_296_970),
        ("2026-06", 3_127_938),
        ("2026-07", 2_112_381),
        ("2026-08", 3_007_736),
        ("2026-09", 7_417_200),
    ];
    for (month, kopecks) in expected {
        let s = ledger.month_summary(ym(month)).unwrap();
        assert_eq!(s.free_actual.kopecks(), kopecks, "{month}");
    }
    let sep = ledger.month_summary(ym("2026-09")).unwrap();
    assert_eq!(sep.free.kopecks(), 4_272_349, "free с планом не изменился");
}

#[test]
fn weekly_in_the_current_month_divides_by_the_remaining_weeks() {
    let data = load_seed(SEED).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    // Сентябрь, 30 дней. Сегодня 15-е: осталось 16 дней = 16/7 недели ≈ 2,3.
    // 4 272 349 коп. × 7 / 16 = 1 869 152,69 → 1 869 153 коп.
    let s = ledger
        .month_summary_at(ym("2026-09"), day("2026-09", 15))
        .unwrap();
    assert_eq!(s.per_week.kopecks(), 1_869_153);
    let weeks = s.weeks_left.unwrap();
    assert!((weeks - 16.0 / 7.0).abs() < 1e-9, "{weeks}");
}

#[test]
fn weekly_counts_today_and_has_at_least_one_day() {
    let data = load_seed(SEED).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    let first = ledger
        .month_summary_at(ym("2026-09"), day("2026-09", 1))
        .unwrap();
    // Весь месяц впереди: 30 дней. 4 272 349 × 7 / 30 = 996 881,43 → 996 881.
    assert_eq!(first.per_week.kopecks(), 996_881);
    let last = ledger
        .month_summary_at(ym("2026-09"), day("2026-09", 30))
        .unwrap();
    // Остался один сегодняшний день: 1/7 недели, остаток × 7.
    assert_eq!(last.per_week.kopecks(), 4_272_349 * 7);
    assert!((last.weeks_left.unwrap() - 1.0 / 7.0).abs() < 1e-9);
}

#[test]
fn weekly_in_a_leap_february_uses_29_days() {
    let mut data = january();
    data.transactions.clear();
    data.debts.clear();
    data.debt_payments.clear();
    data.incomes[0].month = ym("2028-02");
    let ledger = Ledger::new(&data).unwrap();
    // 100 000 ₽, 1 февраля 2028: 29 дней. 10 000 000 × 7 / 29 = 2 413 793,10 → 2 413 793.
    let s = ledger
        .month_summary_at(ym("2028-02"), day("2028-02", 1))
        .unwrap();
    assert_eq!(s.per_week.kopecks(), 2_413_793);
}

#[test]
fn weekly_in_past_and_future_months_keeps_the_old_rule() {
    let data = load_seed(SEED).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    let plain = ledger.month_summary(ym("2026-09")).unwrap();
    // Остаток ÷ 4 (weeks_per_month): 4 272 349 / 4 = 1 068 087,25 → 1 068 087.
    assert_eq!(plain.per_week.kopecks(), 1_068_087);
    assert_eq!(plain.weeks_left, None);
    // «Сегодня» в октябре: сентябрь прошлый, август тоже; ноябрь будущий.
    for (month, today) in [
        ("2026-09", day("2026-10", 15)),
        ("2026-08", day("2026-10", 15)),
        ("2026-12", day("2026-10", 15)),
    ] {
        let at = ledger.month_summary_at(ym(month), today).unwrap();
        let old = ledger.month_summary(ym(month)).unwrap();
        assert_eq!(at, old, "{month}");
        assert_eq!(at.weeks_left, None, "{month}");
    }
}

#[test]
fn weekly_negative_free_is_rounded_away_from_zero() {
    let mut data = january();
    data.incomes[0].amount = Money::from_kopecks(1);
    data.debts.clear();
    data.debt_payments.clear();
    data.transactions.retain(|t| t.id == TxId(1));
    // free = 1 − 500 000 = −499 999 коп.; 15-е января: 17 дней; −499 999 × 7 / 17 = −205 881,94
    // → −205 882 (half away from zero).
    let s = Ledger::new(&data)
        .unwrap()
        .month_summary_at(ym("2026-01"), day("2026-01", 15))
        .unwrap();
    assert_eq!(s.free.kopecks(), -499_999);
    assert_eq!(s.per_week.kopecks(), -205_882);
}

#[test]
fn calendar_day_rejects_impossible_dates() {
    assert!(CalendarDay::new(ym("2026-09"), 0).is_err());
    assert!(CalendarDay::new(ym("2026-09"), 31).is_err());
    assert!(CalendarDay::new(ym("2027-02"), 29).is_err());
    assert!(CalendarDay::new(ym("2028-02"), 29).is_ok());
}

#[test]
fn month_saved_on_the_overview_is_what_the_savings_screen_shows() {
    // «Отложено» на «Обзоре» (plan.saved) и на «Сбережениях» (deposited) — одно и то же:
    // оплаченные записи накоплений без статуса «План». В сентябре seed все накопления в плане:
    // обе величины 0.
    let data = load_seed(SEED).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    for m in 1..=9 {
        let month = YearMonth::new(2026, m).unwrap();
        let saved = ledger.month_plan(month).unwrap().saved;
        let progress = ledger.savings_progress(month).unwrap();
        let deposited = progress
            .months
            .iter()
            .find(|p| p.month == month)
            .map_or(Money::ZERO, |p| p.deposited);
        assert_eq!(saved, deposited, "{month}");
    }
    let sep = ledger.month_plan(ym("2026-09")).unwrap();
    assert_eq!(sep.saved, Money::ZERO);
}

//! План месяца в хранилище: фиксация, разблокировка, копирование, накопления с суммой и
//! параметрами.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::calc::Ledger;
use planning_budget_core::{
    BasisPoints, CategoryId, DebtPaymentStatus, IncomeStatus, Money, SavingsParams, TxId, TxStatus,
    YearMonth,
};
use planning_budget_storage::{
    Db, NewDebt, NewIncome, NewTransaction, RecordSource, StorageError, TransactionPatch,
};
use tempfile::TempDir;

const KEY: [u8; 32] = [9; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn rub(r: i64) -> Money {
    Money::from_kopecks(r * 100)
}

fn seeded() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn cat(db: &Db, name: &str) -> CategoryId {
    db.category_id_by_name(name).unwrap().unwrap()
}

fn spend(db: &mut Db, month: &str, category: CategoryId, amount: i64, status: TxStatus) -> i64 {
    db.transaction_create(
        &NewTransaction {
            month: ym(month),
            date: None,
            category_id: category,
            title: format!("Запись {amount}"),
            amount: rub(amount),
            status,
            comment: None,
            source: RecordSource::Manual,
        },
        now(),
    )
    .unwrap()
    .id
    .0
}

fn income(db: &mut Db, month: &str, amount: i64) {
    db.income_create(
        &NewIncome {
            month: ym(month),
            date: None,
            source_name: "Зарплата".to_owned(),
            amount: rub(amount),
            status: IncomeStatus::Received,
            comment: None,
            source: RecordSource::Manual,
        },
        now(),
    )
    .unwrap();
}

fn plan_of(db: &Db, month: &str) -> planning_budget_core::calc::MonthPlan {
    let data = db.dataset(ym(month), ym(month)).unwrap();
    Ledger::new(&data).unwrap().month_plan(ym(month)).unwrap()
}

/// Октябрь: доход 100 000 ₽, продукты 30 000 ₽ оплачено и 5 000 ₽ в плане, развлечения 3 000 ₽
/// «Незапланировано»; план сбережений — 14 % от дохода из сида.
fn october(db: &mut Db) {
    income(db, "2026-10", 100_000);
    let food = cat(db, "Продукты");
    spend(db, "2026-10", food, 30_000, TxStatus::Paid);
    spend(db, "2026-10", food, 5_000, TxStatus::Planned);
    let fun = cat(db, "Хобби и игры");
    spend(db, "2026-10", fun, 3_000, TxStatus::Unplanned);
}

#[test]
fn lock_freezes_the_plan_and_unlock_releases_it() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let before = plan_of(&db, "2026-10");
    assert!(!before.locked);
    assert_eq!(before.plan_expenses, rub(35_000));

    db.plan_lock(ym("2026-10"), now()).unwrap();
    assert!(db.plan_is_locked(ym("2026-10")).unwrap());
    let locked = plan_of(&db, "2026-10");
    assert!(locked.locked);
    assert_eq!(locked.plan_expenses, rub(35_000));
    assert_eq!(locked.plan_savings, rub(14_000));
    assert_eq!(locked.unallocated, before.unallocated);

    db.plan_unlock(ym("2026-10")).unwrap();
    assert!(!plan_of(&db, "2026-10").locked);
}

#[test]
fn locking_twice_or_unlocking_a_free_month_is_a_conflict() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    assert!(matches!(
        db.plan_unlock(ym("2026-10")),
        Err(StorageError::Conflict("plan.not_locked"))
    ));
    db.plan_lock(ym("2026-10"), now()).unwrap();
    assert!(matches!(
        db.plan_lock(ym("2026-10"), now()),
        Err(StorageError::Conflict("plan.already_locked"))
    ));
}

#[test]
fn locked_plan_keeps_planned_sums_after_a_fact_changes() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let food = cat(&db, "Продукты");
    db.plan_lock(ym("2026-10"), now()).unwrap();

    let paid_id = db
        .transactions_of_month(ym("2026-10"))
        .unwrap()
        .into_iter()
        .find(|t| t.amount == rub(30_000))
        .unwrap()
        .id;
    db.transaction_update(
        paid_id,
        &TransactionPatch {
            amount: Some(rub(32_000)),
            ..TransactionPatch::default()
        },
        now(),
    )
    .unwrap();
    let plan = plan_of(&db, "2026-10");
    let row = plan.rows.iter().find(|r| r.category_id == food).unwrap();
    assert_eq!(row.plan, rub(35_000), "план остался прежним");
    assert_eq!(row.fact, rub(37_000));
    assert_eq!(row.deviation, rub(2_000));
}

#[test]
fn relock_after_unlock_takes_the_new_amounts() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    db.plan_lock(ym("2026-10"), now()).unwrap();
    db.plan_unlock(ym("2026-10")).unwrap();
    let food = cat(&db, "Продукты");
    spend(&mut db, "2026-10", food, 4_000, TxStatus::Planned);
    db.plan_lock(ym("2026-10"), now()).unwrap();
    assert_eq!(plan_of(&db, "2026-10").plan_expenses, rub(39_000));
}

#[test]
fn lock_snapshots_savings_and_repayments() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let clothes = cat(&db, "Одежда");
    let debt = db
        .debt_create(
            &NewDebt {
                lender: "Карта".to_owned(),
                amount: rub(10_000),
                taken_month: ym("2026-09"),
                taken_date: None,
                category_id: Some(clothes),
                transaction_id: None,
                comment: None,
                schedule: vec![(ym("2026-10"), rub(10_000))],
            },
            now(),
        )
        .unwrap();
    db.plan_lock(ym("2026-10"), now()).unwrap();
    // Позже график и доход меняются, а зафиксированный план — нет.
    db.debt_payment_set_status(debt.payments[0].id, DebtPaymentStatus::Paid, None, now())
        .unwrap();
    income(&mut db, "2026-10", 100_000);
    let plan = plan_of(&db, "2026-10");
    assert_eq!(plan.plan_repayments, rub(10_000));
    assert_eq!(plan.plan_savings, rub(14_000));
}

#[test]
fn copy_creates_planned_rows_in_the_next_month() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let created = db
        .plan_copy_from(ym("2026-10"), ym("2026-11"), now())
        .unwrap();
    // Оплаченные продукты и плановые продукты; «Незапланировано» не копируется.
    assert_eq!(created, 2);
    let rows = db.transactions_of_month(ym("2026-11")).unwrap();
    assert!(rows.iter().all(|t| t.status == TxStatus::Planned));
    let total: i64 = rows.iter().map(|t| t.amount.kopecks()).sum();
    assert_eq!(total, rub(35_000).kopecks());
}

#[test]
fn copy_refuses_non_empty_locked_or_same_month() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    db.plan_copy_from(ym("2026-10"), ym("2026-11"), now())
        .unwrap();
    assert!(matches!(
        db.plan_copy_from(ym("2026-10"), ym("2026-11"), now()),
        Err(StorageError::Conflict("plan.not_empty"))
    ));
    assert!(matches!(
        db.plan_copy_from(ym("2026-10"), ym("2026-10"), now()),
        Err(StorageError::Invalid("plan.same_month"))
    ));
    db.plan_lock(ym("2026-12"), now()).unwrap();
    assert!(matches!(
        db.plan_copy_from(ym("2026-10"), ym("2026-12"), now()),
        Err(StorageError::Conflict("plan.locked"))
    ));
}

#[test]
fn copy_skips_archived_categories() {
    let (_dir, mut db) = seeded();
    income(&mut db, "2026-10", 100_000);
    let hobby = cat(&db, "Хобби и игры");
    spend(&mut db, "2026-10", hobby, 1_000, TxStatus::Planned);
    let food = cat(&db, "Продукты");
    spend(&mut db, "2026-10", food, 2_000, TxStatus::Planned);
    db.category_archive(hobby, ym("2026-10"), now()).unwrap();
    assert_eq!(
        db.plan_copy_from(ym("2026-10"), ym("2026-11"), now())
            .unwrap(),
        1
    );
}

#[test]
fn fixed_savings_plan_and_params_reach_the_dataset() {
    let (_dir, mut db) = seeded();
    income(&mut db, "2026-10", 100_000);
    let savings = cat(&db, "Сбережения");
    db.savings_fixed_set(savings, ym("2026-10"), rub(5_000))
        .unwrap();
    let data = db.dataset(ym("2026-10"), ym("2026-10")).unwrap();
    let ledger = Ledger::new(&data).unwrap();
    assert_eq!(
        ledger
            .category_savings_plan(ym("2026-10"), savings)
            .unwrap(),
        rub(5_000)
    );

    // Возврат к проценту сбрасывает фиксированную сумму.
    db.savings_rate_set(savings, ym("2026-10"), BasisPoints(1000))
        .unwrap();
    let data = db.dataset(ym("2026-10"), ym("2026-10")).unwrap();
    assert_eq!(
        Ledger::new(&data)
            .unwrap()
            .category_savings_plan(ym("2026-10"), savings)
            .unwrap(),
        rub(10_000)
    );
}

#[test]
fn seeded_savings_category_has_default_params_and_new_ones_start_plain() {
    let (_dir, mut db) = seeded();
    let seeded_cat = cat(&db, "Сбережения");
    let params = db.savings_params().unwrap();
    let p = params[&seeded_cat];
    assert_eq!(
        (p.annual_rate, p.tax),
        (BasisPoints(1600), BasisPoints(1300))
    );

    let new = db
        .category_create(
            &planning_budget_storage::NewCategory {
                name: "Отпуск".to_owned(),
                kind: planning_budget_core::CategoryKind::Savings,
                color: "#3AA567".to_owned(),
                note: None,
            },
            now(),
        )
        .unwrap();
    let params = db.savings_params().unwrap();
    assert_eq!(params[&new.id].annual_rate, BasisPoints(0));
    assert_eq!(params[&new.id].initial_month, ym("2026-10"));

    let updated = SavingsParams {
        annual_rate: BasisPoints(900),
        tax: BasisPoints(0),
        initial_balance: rub(20_000),
        initial_month: ym("2026-01"),
    };
    db.savings_params_set(new.id, &updated).unwrap();
    assert_eq!(db.savings_params().unwrap()[&new.id], updated);

    let food = cat(&db, "Продукты");
    assert!(
        db.savings_params_set(food, &updated).is_err(),
        "только накопления"
    );
}

fn wizard_input(db: &Db) -> planning_budget_storage::PlanWizardInput {
    use planning_budget_storage::SavingsPlanInput::Percent;
    planning_budget_storage::PlanWizardInput {
        incomes: vec![("Зарплата".to_owned(), rub(100_000))],
        lines: vec![
            (cat(db, "Продукты"), rub(30_000)),
            (cat(db, "Кафе и доставка"), rub(10_000)),
        ],
        savings: vec![(cat(db, "Сбережения"), Percent(BasisPoints(1000)))],
    }
}

#[test]
fn wizard_creates_expected_income_planned_lines_and_savings_then_locks() {
    let (_dir, mut db) = seeded();
    let input = wizard_input(&db);
    db.plan_wizard_apply(ym("2026-10"), &input, now()).unwrap();

    let plan = plan_of(&db, "2026-10");
    assert!(plan.locked);
    assert_eq!(plan.income, rub(100_000));
    assert_eq!(plan.plan_expenses, rub(40_000));
    assert_eq!(plan.plan_savings, rub(10_000));
    assert_eq!(plan.unallocated, rub(50_000));

    let rows = db.transactions_of_month(ym("2026-10")).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|t| t.status == TxStatus::Planned));
    let incomes = db.incomes_of_month(ym("2026-10")).unwrap();
    assert_eq!(incomes.len(), 1);
    assert_eq!(incomes[0].status, IncomeStatus::Expected);
}

#[test]
fn wizard_with_a_fixed_savings_plan_and_override_cleanup() {
    use planning_budget_storage::SavingsPlanInput::Fixed;
    let (_dir, mut db) = seeded();
    let savings = cat(&db, "Сбережения");
    db.savings_override_set(ym("2026-10"), savings, BasisPoints(2000))
        .unwrap();
    let mut input = wizard_input(&db);
    input.savings = vec![(savings, Fixed(rub(7_000)))];
    db.plan_wizard_apply(ym("2026-10"), &input, now()).unwrap();
    assert_eq!(plan_of(&db, "2026-10").plan_savings, rub(7_000));
}

#[test]
fn wizard_refuses_a_locked_or_already_planned_month() {
    let (_dir, mut db) = seeded();
    let input = wizard_input(&db);
    db.plan_wizard_apply(ym("2026-10"), &input, now()).unwrap();
    assert!(matches!(
        db.plan_wizard_apply(ym("2026-10"), &input, now()),
        Err(StorageError::Conflict("plan.locked"))
    ));
    db.plan_unlock(ym("2026-10")).unwrap();
    assert!(matches!(
        db.plan_wizard_apply(ym("2026-10"), &input, now()),
        Err(StorageError::Conflict("plan.not_empty"))
    ));
}

#[test]
fn wizard_with_bad_input_writes_nothing() {
    use planning_budget_storage::SavingsPlanInput::Percent;
    let (_dir, mut db) = seeded();
    // Накопление в статьях расходов.
    let mut input = wizard_input(&db);
    input.lines.push((cat(&db, "Сбережения"), rub(1_000)));
    assert!(matches!(
        db.plan_wizard_apply(ym("2026-10"), &input, now()),
        Err(StorageError::Invalid("plan.line_is_savings"))
    ));
    // Статья расходов в накоплениях.
    let mut input = wizard_input(&db);
    input.savings = vec![(cat(&db, "Продукты"), Percent(BasisPoints(500)))];
    assert!(db.plan_wizard_apply(ym("2026-10"), &input, now()).is_err());
    // Процент вне 0–100 % и пустое название дохода.
    let mut input = wizard_input(&db);
    input.savings = vec![(cat(&db, "Сбережения"), Percent(BasisPoints(10_001)))];
    assert!(db.plan_wizard_apply(ym("2026-10"), &input, now()).is_err());
    let mut input = wizard_input(&db);
    input.incomes = vec![("  ".to_owned(), rub(1))];
    assert!(db.plan_wizard_apply(ym("2026-10"), &input, now()).is_err());

    assert!(db.transactions_of_month(ym("2026-10")).unwrap().is_empty());
    assert!(db.incomes_of_month(ym("2026-10")).unwrap().is_empty());
    assert!(!db.plan_is_locked(ym("2026-10")).unwrap());
}

fn december() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 15, 12, 0, 0).unwrap()
}

// ---- U-1: автоперенос плана на новый месяц ----

#[test]
fn carry_forward_copies_from_the_latest_month_with_data_even_across_a_gap() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    // Ноябрь пропущен: декабрь берёт план из октября.
    let created = db
        .plan_carry_forward(ym("2026-12"), ym("2026-12"), december())
        .unwrap();
    assert_eq!(created, 2);
    let rows = db.transactions_of_month(ym("2026-12")).unwrap();
    assert!(rows.iter().all(|t| t.status == TxStatus::Planned));
    let total: i64 = rows.iter().map(|t| t.amount.kopecks()).sum();
    assert_eq!(total, rub(35_000).kopecks());
}

#[test]
fn carry_forward_happens_once_per_month_even_if_the_user_clears_it() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    assert_eq!(
        db.plan_carry_forward(ym("2026-11"), ym("2026-12"), december())
            .unwrap(),
        2
    );
    for row in db.transactions_of_month(ym("2026-11")).unwrap() {
        db.transaction_delete(row.id, now()).unwrap();
    }
    assert_eq!(
        db.plan_carry_forward(ym("2026-11"), ym("2026-12"), december())
            .unwrap(),
        0
    );
    assert!(db.transactions_of_month(ym("2026-11")).unwrap().is_empty());
}

#[test]
fn carry_forward_leaves_a_month_that_already_has_records() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let food = cat(&db, "Продукты");
    spend(&mut db, "2026-11", food, 1_000, TxStatus::Paid);
    assert_eq!(
        db.plan_carry_forward(ym("2026-11"), ym("2026-12"), december())
            .unwrap(),
        0
    );
    assert_eq!(db.transactions_of_month(ym("2026-11")).unwrap().len(), 1);
}

#[test]
fn carry_forward_leaves_a_locked_month_and_the_first_month_alone() {
    let (_dir, mut db) = seeded();
    // Данных ещё нет: переносить нечего, и отметки «перенесено» не остаётся.
    assert_eq!(
        db.plan_carry_forward(ym("2026-10"), ym("2026-12"), december())
            .unwrap(),
        0
    );
    october(&mut db);
    db.plan_lock(ym("2026-11"), now()).unwrap();
    assert_eq!(
        db.plan_carry_forward(ym("2026-11"), ym("2026-12"), december())
            .unwrap(),
        0
    );
    assert!(db.transactions_of_month(ym("2026-11")).unwrap().is_empty());
    // Прошлое не заполняется из будущего.
    assert_eq!(
        db.plan_carry_forward(ym("2026-09"), ym("2026-12"), december())
            .unwrap(),
        0
    );
}

#[test]
fn carry_forward_does_not_copy_savings_and_skips_archived_categories() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let savings = cat(&db, "Сбережения");
    spend(&mut db, "2026-10", savings, 14_000, TxStatus::Paid);
    let fun = cat(&db, "Хобби и игры");
    spend(&mut db, "2026-10", fun, 2_000, TxStatus::Planned);
    db.category_archive(fun, ym("2026-10"), now()).unwrap();
    db.plan_carry_forward(ym("2026-11"), ym("2026-12"), december())
        .unwrap();
    let rows = db.transactions_of_month(ym("2026-11")).unwrap();
    assert!(!rows.is_empty());
    assert!(
        rows.iter()
            .all(|t| t.category_id != savings && t.category_id != fun)
    );
}

#[test]
fn carry_forward_ignores_future_months_and_keeps_them_for_later() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    // «Сегодня» — декабрь 2026: январь 2030 ещё не наступил, отметка не ставится.
    assert_eq!(
        db.plan_carry_forward(ym("2030-01"), ym("2026-12"), december())
            .unwrap(),
        0
    );
    assert!(db.transactions_of_month(ym("2030-01")).unwrap().is_empty());
    // Когда месяц наступил, план переносится как обычно.
    let later = Utc.with_ymd_and_hms(2030, 1, 5, 9, 0, 0).unwrap();
    assert_eq!(
        db.plan_carry_forward(ym("2030-01"), ym("2030-01"), later)
            .unwrap(),
        2
    );
}

#[test]
fn carry_forward_does_not_fill_a_month_older_than_existing_data() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    let food = cat(&db, "Продукты");
    spend(&mut db, "2026-08", food, 9_000, TxStatus::Paid);
    // Сентябрь лежит между августом и октябрём: «последнего» месяца для него нет.
    assert_eq!(
        db.plan_carry_forward(ym("2026-09"), ym("2026-12"), december())
            .unwrap(),
        0
    );
    assert!(db.transactions_of_month(ym("2026-09")).unwrap().is_empty());
}

// ---- U-3: начало накоплений для расчёта «за весь период» ----

#[test]
fn first_deposit_month_is_the_earliest_non_planned_savings_record() {
    let (_dir, mut db) = seeded();
    assert_eq!(db.savings_first_deposit_month().unwrap(), None);
    let savings = cat(&db, "Сбережения");
    let food = cat(&db, "Продукты");
    spend(&mut db, "2025-01", food, 1_000, TxStatus::Paid);
    spend(&mut db, "2025-02", savings, 1_000, TxStatus::Planned);
    assert_eq!(db.savings_first_deposit_month().unwrap(), None);
    spend(&mut db, "2025-11", savings, 5_000, TxStatus::Paid);
    spend(&mut db, "2026-03", savings, 5_000, TxStatus::Paid);
    assert_eq!(
        db.savings_first_deposit_month().unwrap(),
        Some(ym("2025-11"))
    );
}

#[test]
fn wizard_refuses_a_month_filled_by_the_automatic_carry_forward() {
    // U-1: открытие нового месяца переносит план прошлого; мастер заполняет только пустой месяц.
    let (_dir, mut db) = seeded();
    let food = cat(&db, "Продукты");
    spend(&mut db, "2026-09", food, 30_000, TxStatus::Planned);
    assert!(
        db.plan_carry_forward(ym("2026-10"), ym("2026-12"), december())
            .unwrap()
            > 0
    );
    let input = wizard_input(&db);
    assert!(matches!(
        db.plan_wizard_apply(ym("2026-10"), &input, now()),
        Err(StorageError::Conflict("plan.not_empty"))
    ));
}

// ---- Q-17: сбережения по месяцам — несколько сумм одного накопления ----

#[test]
fn several_amounts_of_one_savings_category_add_up_in_the_month() {
    // 30 000 ₽ откладываются тремя разами по 10 000 ₽: это три суммы одного накопления.
    let (_dir, mut db) = seeded();
    let savings = cat(&db, "Сбережения");
    let ids: Vec<i64> = (0..3)
        .map(|_| spend(&mut db, "2026-10", savings, 10_000, TxStatus::Paid))
        .collect();
    let total = |db: &Db| {
        let data = db.dataset(ym("2026-10"), ym("2026-10")).unwrap();
        Ledger::new(&data)
            .unwrap()
            .category_total(ym("2026-10"), savings)
            .unwrap()
    };
    assert_eq!(total(&db), rub(30_000));
    assert_eq!(db.transactions_of_month(ym("2026-10")).unwrap().len(), 3);

    // Одну сумму можно изменить, другую удалить; остальные остаются.
    db.transaction_update(
        TxId(ids[0]),
        &TransactionPatch {
            amount: Some(rub(15_000)),
            ..TransactionPatch::default()
        },
        now(),
    )
    .unwrap();
    db.transaction_delete(TxId(ids[2]), now()).unwrap();
    assert_eq!(total(&db), rub(25_000));

    // Другой месяц не затронут: каждый месяц начинается пустым.
    let data = db.dataset(ym("2026-11"), ym("2026-11")).unwrap();
    assert_eq!(
        Ledger::new(&data)
            .unwrap()
            .category_total(ym("2026-11"), savings)
            .unwrap(),
        Money::ZERO
    );
}

#[test]
fn carry_forward_uses_the_local_month_not_the_utc_one() {
    let (_dir, mut db) = seeded();
    october(&mut db);
    // 31 октября 17:00 UTC: в UTC+7 уже 1 ноября, поэтому ноябрь наступил и план переносится.
    let utc = Utc.with_ymd_and_hms(2026, 10, 31, 17, 0, 0).unwrap();
    assert_eq!(
        db.plan_carry_forward(ym("2026-11"), ym("2026-11"), utc)
            .unwrap(),
        2
    );
}

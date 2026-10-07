//! Поиск: текст через FTS5, фильтр через параметризованный SQL, группы результатов.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::query::{Filter, parse};
use planning_budget_core::{CategoryId, IncomeStatus, Money, TxStatus, YearMonth};
use planning_budget_storage::{Db, NewIncome, NewTransaction, RecordSource, SearchResult};
use tempfile::TempDir;

const KEY: [u8; 32] = [6; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

fn cat(db: &Db, name: &str) -> CategoryId {
    db.category_id_by_name(name).unwrap().unwrap()
}

fn add_tx(
    db: &mut Db,
    category: &str,
    month: &str,
    title: &str,
    rub: i64,
    status: TxStatus,
) -> i64 {
    let category_id = cat(db, category);
    db.transaction_create(
        &NewTransaction {
            month: ym(month),
            date: None,
            category_id,
            title: title.to_owned(),
            amount: Money::from_kopecks(rub * 100),
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

fn add_income(db: &mut Db, month: &str, name: &str, rub: i64) {
    db.income_create(
        &NewIncome {
            month: ym(month),
            date: None,
            source_name: name.to_owned(),
            amount: Money::from_kopecks(rub * 100),
            status: IncomeStatus::Received,
            comment: None,
            source: RecordSource::Manual,
        },
        now(),
    )
    .unwrap();
}

/// Небольшой бюджет, на котором проверяются все токены языка запроса.
fn fixture() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    add_tx(
        &mut db,
        "Продукты",
        "2026-06",
        "Лента",
        4_000,
        TxStatus::Paid,
    );
    add_tx(
        &mut db,
        "Продукты",
        "2026-09",
        "Лента",
        12_000,
        TxStatus::Paid,
    );
    add_tx(
        &mut db,
        "Продукты",
        "2026-09",
        "Магнит",
        800,
        TxStatus::Planned,
    );
    add_tx(
        &mut db,
        "Подписки",
        "2026-09",
        "Ёлка онлайн",
        300,
        TxStatus::Debt,
    );
    let gift = add_tx(
        &mut db,
        "Подарки и праздники",
        "2026-08",
        "Подарок коллеге",
        2_500,
        TxStatus::Paid,
    );
    let tag = db.tag_create("Подарки").unwrap();
    db.transaction_tags_set(planning_budget_core::TxId(gift), &[tag.id])
        .unwrap();
    add_income(&mut db, "2026-09", "Зарплата 14", 60_000);
    add_income(&mut db, "2026-08", "Подработка", 9_000);
    (dir, db)
}

fn run(db: &Db, query: &str) -> SearchResult {
    let q = parse(query, 2026);
    db.search(&q.text, &q.filter, 8).unwrap()
}

fn titles(r: &SearchResult) -> Vec<&str> {
    r.transactions
        .items
        .iter()
        .map(|t| t.title.as_str())
        .collect()
}

#[test]
fn empty_query_returns_everything_newest_first() {
    let (_d, db) = fixture();
    let r = run(&db, "");
    assert_eq!(r.transactions.total, 5);
    assert_eq!(r.incomes.total, 2);
    assert_eq!(
        r.transactions.items.first().map(|t| t.month),
        Some(ym("2026-09"))
    );
    assert!(r.categories.is_empty() && r.months.is_empty());
}

#[test]
fn text_matches_by_prefix_case_and_yo() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "лен").transactions.total, 2);
    assert_eq!(run(&db, "ЕЛКА").transactions.total, 1);
    assert_eq!(
        run(&db, "елка")
            .transactions
            .items
            .first()
            .map(|t| t.title.as_str()),
        Some("Ёлка онлайн")
    );
}

#[test]
fn text_searches_category_and_tags() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "подписки").transactions.total, 1);
    assert_eq!(run(&db, "подарки").transactions.total, 1);
}

#[test]
fn phrase_requires_adjacent_words() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "\"подарок коллеге\"").transactions.total, 1);
    assert_eq!(run(&db, "\"коллеге подарок\"").transactions.total, 0);
}

#[test]
fn group_total_and_sum_cover_all_matches_not_only_the_page() {
    let (_d, db) = fixture();
    let q = parse("лента", 2026);
    let r = db.search(&q.text, &q.filter, 1).unwrap();
    assert_eq!(r.transactions.items.len(), 1);
    assert_eq!(r.transactions.total, 2);
    assert_eq!(r.transactions.sum, Money::from_kopecks(1_600_000));
}

#[test]
fn doc_example_filters_combine() {
    let (_d, db) = fixture();
    let r = run(&db, "лента статус:оплачено период:июн..сен сумма>10000");
    assert_eq!(titles(&r), ["Лента"]);
    assert_eq!(r.transactions.total, 1);
}

#[test]
fn amount_bounds_are_inclusive_and_strict_operators_shift_a_kopeck() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "сумма:800..4000").transactions.total, 3);
    assert_eq!(run(&db, "сумма>800").transactions.total, 3);
    assert_eq!(run(&db, "сумма<=800").transactions.total, 2);
}

#[test]
fn category_prefix_kind_and_negation() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "кат:прод").transactions.total, 3);
    assert_eq!(
        run(&db, "кат:\"подарки и праздники\"").transactions.total,
        1
    );
    assert_eq!(run(&db, "тип:желания").transactions.total, 2);
    assert_eq!(run(&db, "-кат:продукты").transactions.total, 2);
    assert_eq!(run(&db, "кат:несуществует").transactions.total, 0);
}

#[test]
fn tags_include_exclude_and_unknown() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "#подарки").transactions.total, 1);
    assert_eq!(run(&db, "-#подарки").transactions.total, 4);
    assert_eq!(run(&db, "#нетТакого").transactions.total, 0);
}

#[test]
fn record_kind_and_category_filters_scope_incomes() {
    let (_d, db) = fixture();
    let only_income = run(&db, "доход");
    assert_eq!(
        (only_income.transactions.total, only_income.incomes.total),
        (0, 2)
    );
    let only_expense = run(&db, "расход");
    assert_eq!(
        (only_expense.transactions.total, only_expense.incomes.total),
        (5, 0)
    );
    assert_eq!(run(&db, "кат:продукты").incomes.total, 0);
    assert_eq!(run(&db, "зарплата").incomes.total, 1);
    assert_eq!(run(&db, "мес:авг").incomes.total, 1);
}

#[test]
fn source_filter_uses_record_source() {
    let (_d, db) = fixture();
    assert_eq!(run(&db, "src:вручную").transactions.total, 5);
    assert_eq!(run(&db, "src:импорт").transactions.total, 0);
}

#[test]
fn text_finds_categories_and_months() {
    let (_d, db) = fixture();
    let r = run(&db, "хобби");
    assert_eq!(
        r.categories
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Хобби и игры"]
    );
    let r = run(&db, "сен");
    assert_eq!(
        r.months
            .iter()
            .map(|m| (m.month, m.records))
            .collect::<Vec<_>>(),
        [(ym("2026-09"), 4)]
    );
}

#[test]
fn soft_deleted_and_renamed_rows_follow_the_index() {
    let (_d, mut db) = fixture();
    let id = add_tx(
        &mut db,
        "Спорт",
        "2026-09",
        "Бассейн",
        1_000,
        TxStatus::Paid,
    );
    assert_eq!(run(&db, "бассейн").transactions.total, 1);
    db.transaction_delete(planning_budget_core::TxId(id), now())
        .unwrap();
    assert_eq!(run(&db, "бассейн").transactions.total, 0);
    db.transaction_restore(planning_budget_core::TxId(id), now())
        .unwrap();
    assert_eq!(run(&db, "бассейн").transactions.total, 1);
}

#[test]
fn hostile_input_is_data_not_syntax() {
    let (_d, db) = fixture();
    for q in [
        "\"",
        "* OR 1=1",
        "лента\" OR \"",
        "кат:'; DROP TABLE categories;--",
        "NEAR(",
        "-",
        "()",
    ] {
        let r = run(&db, q);
        assert!(r.transactions.total <= 5, "{q}");
    }
    assert_eq!(run(&db, "").transactions.total, 5);
}

#[test]
fn filter_alone_with_default_filter_matches_all() {
    let (_d, db) = fixture();
    let r = db.search(&[], &Filter::default(), 3).unwrap();
    assert_eq!(r.transactions.total, 5);
    assert_eq!(r.transactions.items.len(), 3);
}

#[test]
fn transaction_list_returns_full_records_with_tags_and_real_total() {
    let (_d, db) = fixture();
    let q = parse("статус:оплачено", 2026);
    let all = db.transaction_search(&q.text, &q.filter, 100).unwrap();
    assert_eq!((all.total, all.items.len()), (3, 3));
    assert_eq!(all.sum, Money::from_kopecks(1_850_000));
    let gift = all
        .items
        .iter()
        .find(|t| t.title == "Подарок коллеге")
        .unwrap();
    assert_eq!(gift.tags.len(), 1);
    assert_eq!(all.items.first().map(|t| t.month), Some(ym("2026-09")));

    let capped = db.transaction_search(&q.text, &q.filter, 2).unwrap();
    assert_eq!((capped.total, capped.items.len()), (3, 2));
}

#[test]
fn income_list_ignores_expense_only_filters() {
    let (_d, db) = fixture();
    let q = parse("зарплата", 2026);
    let found = db.income_search(&q.text, &q.filter, 10).unwrap();
    assert_eq!(found.items.len(), 1);
    assert_eq!(found.sum, Money::from_kopecks(6_000_000));
    let with_status = parse("статус:оплачено", 2026);
    let none = db
        .income_search(&with_status.text, &with_status.filter, 10)
        .unwrap();
    assert_eq!(none.total, 0);
    let expenses_only = parse("расход", 2026);
    assert!(
        db.income_search(&expenses_only.text, &expenses_only.filter, 10)
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        db.transaction_search(&[], &parse("доход", 2026).filter, 10)
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn saved_filters_upsert_by_name_and_validate() {
    let (_d, mut db) = fixture();
    let first = db
        .saved_filter_save(" Крупное ", "сумма>10000", now())
        .unwrap();
    assert_eq!(first.name, "Крупное");
    let again = db
        .saved_filter_save("крупное", "сумма>20000", now())
        .unwrap();
    assert_eq!(again.id, first.id);
    db.saved_filter_save("Подарки", "#подарки", now()).unwrap();
    let all = db.saved_filters().unwrap();
    assert_eq!(
        all.iter()
            .map(|f| (f.name.as_str(), f.query.as_str()))
            .collect::<Vec<_>>(),
        [("крупное", "сумма>20000"), ("Подарки", "#подарки")]
    );
    assert!(matches!(
        db.saved_filter_save(" ", "x", now()),
        Err(planning_budget_storage::StorageError::Invalid(_))
    ));
    assert!(matches!(
        db.saved_filter_save("a", "  ", now()),
        Err(planning_budget_storage::StorageError::Invalid(_))
    ));
    db.saved_filter_delete(first.id).unwrap();
    assert!(matches!(
        db.saved_filter_delete(first.id),
        Err(planning_budget_storage::StorageError::NotFound)
    ));
}

#[test]
fn title_suggestions_come_from_all_history_most_used_first() {
    let (_d, mut db) = fixture();
    add_tx(
        &mut db,
        "Кафе и доставка",
        "2026-03",
        "Лента кафе",
        300,
        TxStatus::Paid,
    );
    let found = db.title_suggestions("лен", 5).unwrap();
    let titles: Vec<_> = found.iter().map(|s| (s.title.as_str(), s.uses)).collect();
    assert_eq!(titles, [("Лента", 2), ("Лента кафе", 1)]);
    assert_eq!(found[0].category_id, cat(&db, "Продукты"));
    // Точное совпадение не предлагается; слова ищутся только в названии, не в категории.
    assert!(
        db.title_suggestions("Лента", 5)
            .unwrap()
            .iter()
            .all(|s| s.title != "Лента")
    );
    assert!(db.title_suggestions("подписки", 5).unwrap().is_empty());
    assert!(db.title_suggestions("  ", 5).unwrap().is_empty());
}

#[test]
fn category_usage_counts_from_the_given_month() {
    let (_d, db) = fixture();
    let usage = db.category_usage(ym("2026-09")).unwrap();
    assert_eq!(
        usage,
        vec![(cat(&db, "Продукты"), 2), (cat(&db, "Подписки"), 1)]
    );
}

#[test]
fn text_without_letters_or_digits_matches_nothing_instead_of_everything() {
    let (_d, db) = fixture();
    for q in ["???", "-", "..."] {
        let parsed = parse(q, 2026);
        let palette = db.search(&parsed.text, &parsed.filter, 8).unwrap();
        assert_eq!(
            (palette.transactions.total, palette.incomes.total),
            (0, 0),
            "{q}"
        );
        assert_eq!(
            db.transaction_search(&parsed.text, &parsed.filter, 10)
                .unwrap()
                .total,
            0,
            "{q}"
        );
        assert_eq!(
            db.income_search(&parsed.text, &parsed.filter, 10)
                .unwrap()
                .total,
            0,
            "{q}"
        );
    }
}

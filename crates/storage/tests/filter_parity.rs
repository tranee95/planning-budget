//! Фильтр исполняется дважды: SQL-ом в `transaction_search` и в памяти в аналитике
//! (`core::analytics::Matcher`). Число трат, прошедших один и тот же фильтр, обязано совпадать
//! (аудит 2026-10-15, A3). Категории с двойными пробелами и `ё`, теги, статусы, суммы и месяцы
//! берутся из набора фрагментов, фильтр собирается из одного-трёх случайных.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{DateTime, TimeZone, Utc};
use planning_budget_core::analytics::{
    ChartSpec, ChartType, Context, GroupBy, Metric, Options, PeriodSpec, compute,
};
use planning_budget_core::query::parse;
use planning_budget_core::{CategoryKind, Money, TxStatus, YearMonth};
use planning_budget_storage::{Db, NewCategory, NewTransaction, RecordSource};
use proptest::prelude::*;
use tempfile::TempDir;

const KEY: [u8; 32] = [8; 32];

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn ym(s: &str) -> YearMonth {
    YearMonth::parse(s).unwrap()
}

/// Хранилище с категориями «неудобных» имён и тратами за три месяца.
fn fixture() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.seed_defaults(now()).unwrap();
    let cats = [
        ("Еда  дома", CategoryKind::Mandatory),
        ("Ёлки и подарки", CategoryKind::Wants),
        ("Резерв  Ёж", CategoryKind::Savings),
        ("Заём", CategoryKind::Loans),
        ("Дорога  на работу", CategoryKind::Mandatory),
    ];
    let mut ids = Vec::new();
    for (name, kind) in cats {
        let c = db
            .category_create(
                &NewCategory {
                    name: name.to_owned(),
                    kind,
                    color: "#336699".to_owned(),
                    note: None,
                },
                now(),
            )
            .unwrap();
        ids.push(c.id);
    }
    let tag_a = db.tag_create("Ёж").unwrap().id;
    let tag_b = db.tag_create("поездка").unwrap().id;
    let statuses = [
        TxStatus::Paid,
        TxStatus::Planned,
        TxStatus::Debt,
        TxStatus::Unplanned,
    ];
    let months = ["2026-08", "2026-09", "2026-10"];
    let mut n = 0usize;
    for month in months {
        for (ci, category_id) in ids.iter().enumerate() {
            for (si, status) in statuses.iter().enumerate() {
                n += 1;
                let tx = db
                    .transaction_create(
                        &NewTransaction {
                            month: ym(month),
                            date: None,
                            category_id: *category_id,
                            title: format!("трата {n}"),
                            amount: Money::from_kopecks(
                                i64::try_from((ci * 4 + si + 1) * 50_000).unwrap(),
                            ),
                            status: *status,
                            comment: None,
                            source: RecordSource::Manual,
                        },
                        now(),
                    )
                    .unwrap();
                match n % 3 {
                    0 => db.transaction_tags_set(tx.id, &[tag_a]).unwrap(),
                    1 => db.transaction_tags_set(tx.id, &[tag_a, tag_b]).unwrap(),
                    _ => {}
                }
            }
        }
    }
    (dir, db)
}

const FRAGMENTS: &[&str] = &[
    "кат:еда",
    "кат:\"еда дома\"",
    "кат:\"ЕДА  ДОМА\"",
    "кат:дома",
    "кат:елки",
    "кат:ёлки",
    "-кат:дорога",
    "-кат:еда",
    "кат:резерв",
    "#еж",
    "#ёж",
    "#ЁЖ",
    "-#поездка",
    "#поездка",
    "статус:оплачено",
    "статус:план",
    "тип:обязательные",
    "сумма:>=2000",
    "сумма:..5000",
    "сумма:1000..9000",
    "мес:2026-09",
    "мес:2026-08..2026-09",
];

fn search_count(db: &Db, query: &str) -> u64 {
    let parsed = parse(query, 2026);
    assert!(parsed.errors.is_empty(), "{query}: {:?}", parsed.errors);
    assert!(
        parsed.text.is_empty(),
        "{query}: фильтр без свободного текста"
    );
    db.transaction_search(&parsed.text, &parsed.filter, 10_000)
        .unwrap()
        .total
}

fn analytics_count(db: &Db, query: &str) -> u64 {
    let (from, to) = (ym("2026-08"), ym("2026-10"));
    let data = db.dataset(from, to).unwrap();
    let tx_tags = db.tx_tag_names(from, to).unwrap();
    let spec = ChartSpec {
        version: 1,
        title: "parity".to_owned(),
        chart_type: ChartType::Kpi,
        metric: Metric::Count,
        group_by: GroupBy::None,
        series_by: None,
        metrics: Vec::new(),
        period: PeriodSpec::Range { from, to },
        filter: query.to_owned(),
        options: Options::default(),
    };
    let ctx = Context {
        today: to,
        tx_tags: &tx_tags,
    };
    let chart = compute(&data, &spec, &ctx).unwrap();
    let total: f64 = chart
        .series
        .iter()
        .flat_map(|s| s.values.iter().flatten())
        .sum();
    // Счётчик целый: сравнение через округление без приведения f64 → u64 по `as`.
    format!("{total:.0}").parse().unwrap()
}

#[test]
fn every_fragment_gives_the_same_count_in_search_and_analytics() {
    let (_dir, db) = fixture();
    for query in FRAGMENTS {
        assert_eq!(
            search_count(&db, query),
            analytics_count(&db, query),
            "расхождение поиска и аналитики на фильтре «{query}»"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn random_filters_agree(picks in proptest::collection::vec(0..FRAGMENTS.len(), 1..=3)) {
        let (_dir, db) = fixture();
        let query = picks
            .iter()
            .filter_map(|i| FRAGMENTS.get(*i))
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        prop_assert_eq!(
            search_count(&db, &query),
            analytics_count(&db, &query),
            "расхождение на фильтре «{}»", query
        );
    }
}

/// Все пары фрагментов: случайный набор мог не дойти до пары тегов (так расходились `#поездка #еж`).
#[test]
fn every_pair_of_fragments_gives_the_same_count() {
    let (_dir, db) = fixture();
    for a in FRAGMENTS {
        for b in FRAGMENTS {
            let query = format!("{a} {b}");
            assert_eq!(
                search_count(&db, &query),
                analytics_count(&db, &query),
                "расхождение поиска и аналитики на фильтре «{query}»"
            );
        }
    }
}

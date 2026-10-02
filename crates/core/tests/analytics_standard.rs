//! Стандартный дашборд на `seed-2026.json`: значения против golden и снимки `ChartData`.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use budget_core::analytics::{ChartData, Context, Placement, compute, standard_dashboard};
use budget_core::{DataSet, YearMonth, load_seed};
use serde_json::Value;

const SEED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/seed-2026.json"
));
const GOLDEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/golden-2026.json"
));

const RUB_TOL: f64 = 1.0;
const PERCENT_TOL: f64 = 0.01;

fn dataset() -> DataSet {
    load_seed(SEED).unwrap()
}

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn today() -> YearMonth {
    YearMonth::new(2026, 9).unwrap()
}

fn chart(data: &DataSet, index: usize) -> (Placement, ChartData) {
    let placement = standard_dashboard().swap_remove(index);
    let tags = BTreeMap::new();
    let ctx = Context {
        today: today(),
        tx_tags: &tags,
    };
    let result = compute(data, &placement.spec, &ctx).unwrap();
    (placement, result)
}

fn series<'a>(data: &'a ChartData, name: &str) -> &'a [Option<f64>] {
    &data
        .series
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("нет серии {name}"))
        .values
}

fn month_names() -> Vec<&'static str> {
    YearMonth::months_of_year(2026)
        .unwrap()
        .take(9)
        .map(YearMonth::month_name_ru)
        .collect()
}

fn near(actual: Option<f64>, expected: f64, tol: f64, ctx: &str) {
    let actual = actual.unwrap_or_else(|| panic!("{ctx}: нет значения"));
    assert!(
        (actual - expected).abs() <= tol,
        "{ctx}: {actual} ≠ {expected}"
    );
}

fn text(data: &ChartData) -> String {
    let mut out = format!("unit: {:?}\n", data.unit);
    writeln!(out, "categories: {}", data.categories.join(" | ")).unwrap();
    writeln!(out, "tokens: {}", data.category_tokens.join(" | ")).unwrap();
    for s in &data.series {
        let values: Vec<String> = s
            .values
            .iter()
            .map(|v| v.map_or("—".to_owned(), |v| format!("{v:.2}")))
            .collect();
        writeln!(out, "{} [{}]: {}", s.name, s.color, values.join(" ")).unwrap();
    }
    for line in &data.reference_lines {
        writeln!(out, "ref {}: {:.2}..{:?}", line.name, line.from, line.to).unwrap();
    }
    if let Some(totals) = &data.totals {
        let totals: Vec<String> = totals.iter().map(|t| format!("{t:.2}")).collect();
        writeln!(out, "totals: {}", totals.join(" ")).unwrap();
    }
    out
}

#[test]
fn standard_dashboard_specs_are_valid_and_fit_the_grid() {
    let charts = standard_dashboard();
    assert_eq!(charts.len(), 9);
    for c in &charts {
        c.spec.validate().unwrap();
        assert!(u16::from(c.x) + u16::from(c.w) <= 12, "{}", c.spec.title);
    }
    let json = serde_json::to_string(&charts[0].spec).unwrap();
    let back: budget_core::analytics::ChartSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(back, charts[0].spec);
}

#[test]
fn income_expenses_savings_match_golden_months() {
    let data = dataset();
    let g = golden();
    let (_, c) = chart(&data, 0);
    assert_eq!(c.categories.len(), 9);
    for (i, name) in month_names().into_iter().enumerate() {
        let m = &g["months"][name];
        for (series_name, key) in [
            ("Доходы", "income"),
            ("Расходы", "expenses"),
            ("Сбережения", "savings"),
        ] {
            near(
                series(&c, series_name)[i],
                m[key].as_f64().unwrap(),
                RUB_TOL,
                &format!("{name} {key}"),
            );
        }
    }
}

#[test]
fn cumulative_balance_ends_at_year_totals() {
    let data = dataset();
    let g = golden();
    let (_, c) = chart(&data, 1);
    near(
        *series(&c, "Остаток накопительно").last().unwrap(),
        g["year"]["free"].as_f64().unwrap(),
        RUB_TOL,
        "free_cum",
    );
    near(
        *series(&c, "Сбережения накопительно").last().unwrap(),
        g["year"]["savings"].as_f64().unwrap(),
        RUB_TOL,
        "savings_cum",
    );
}

#[test]
fn statuses_and_kinds_match_golden_months() {
    let data = dataset();
    let g = golden();
    let (_, by_status) = chart(&data, 2);
    let (_, by_kind) = chart(&data, 3);
    for (i, name) in month_names().into_iter().enumerate() {
        let m = &g["months"][name];
        for (label, key) in [
            ("Оплачено", "paid"),
            ("Долг", "debt"),
            ("Внеплановые", "unplanned"),
            ("План", "planned"),
        ] {
            let expected = m["status"][key].as_f64().unwrap();
            let values = by_status.series.iter().find(|s| s.name == label);
            match values {
                Some(s) => near(s.values[i], expected, RUB_TOL, &format!("{name} {key}")),
                None => assert!(expected.abs() < f64::EPSILON, "{name} {key}"),
            }
        }
        for (label, key) in [
            ("Обязательные", "mandatory"),
            ("Желания", "wants"),
            ("Сбережения", "savings"),
            ("Займы", "loans"),
        ] {
            let expected = m["kinds"][key].as_f64().unwrap();
            match by_kind.series.iter().find(|s| s.name == label) {
                Some(s) => near(s.values[i], expected, RUB_TOL, &format!("{name} {key}")),
                None => assert!(expected.abs() < f64::EPSILON, "{name} {key}"),
            }
        }
    }
}

#[test]
fn year_by_category_and_kind_match_golden_sums() {
    let data = dataset();
    let g = golden();
    let (_, by_category) = chart(&data, 4);
    let values = &by_category.series[0].values;
    assert!(
        values.windows(2).all(|w| w[0] >= w[1]),
        "категории по убыванию"
    );
    for (i, name) in by_category.categories.iter().enumerate() {
        let expected: f64 = month_names()
            .into_iter()
            .map(|m| {
                g["months"][m]["categories"][name.as_str()]
                    .as_f64()
                    .unwrap()
            })
            .sum();
        near(values[i], expected, RUB_TOL, name);
    }
    assert!(
        !by_category.categories.iter().any(|c| c == "Сбережения"),
        "категория сбережений не входит в расходы"
    );

    let (_, donut) = chart(&data, 5);
    let total: f64 = donut.totals.as_ref().unwrap().iter().sum();
    let expected: f64 = month_names()
        .into_iter()
        .map(|m| {
            ["mandatory", "wants", "savings", "loans"]
                .iter()
                .map(|k| g["months"][m]["kinds"][*k].as_f64().unwrap())
                .sum::<f64>()
        })
        .sum();
    near(Some(total), expected, RUB_TOL, "donut total");
}

#[test]
fn savings_rate_matches_golden_and_has_corridor() {
    let data = dataset();
    let g = golden();
    let (_, c) = chart(&data, 8);
    for (i, name) in month_names().into_iter().enumerate() {
        near(
            c.series[0].values[i],
            g["months"][name]["savings_rate"].as_f64().unwrap() * 100.0,
            PERCENT_TOL,
            name,
        );
    }
    assert_eq!(c.reference_lines.len(), 1);
    assert!(c.reference_lines[0].to.is_some());
}

#[test]
fn limit_usage_shows_only_limited_non_savings_categories() {
    let data = dataset();
    let (_, c) = chart(&data, 7);
    assert!(!c.categories.iter().any(|n| n == "Сбережения"));
    assert!(c.categories.contains(&"Продукты".to_owned()));
    near(
        c.series[0].values[c.categories.iter().position(|n| n == "Продукты").unwrap()],
        16_840.0 / 20_000.0 * 100.0,
        PERCENT_TOL,
        "Продукты",
    );
    assert_eq!(c.reference_lines.len(), 1);
}

#[test]
fn snapshots_of_the_standard_dashboard() {
    let data = dataset();
    const SLUGS: [&str; 9] = [
        "income_expenses_savings",
        "balance",
        "by_status",
        "by_kind",
        "by_category",
        "structure",
        "large_categories",
        "limit_and_fact",
        "savings_rate",
    ];
    for (index, slug) in SLUGS.iter().enumerate() {
        let (_, c) = chart(&data, index);
        let name = format!("{}_{slug}", index + 1);
        insta::assert_snapshot!(name, text(&c));
    }
}

//! Валидация `ChartSpec` и параметры движка на `seed-2026.json`.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::analytics::{
    ChartSpec, ChartType, Context, GroupBy, Metric, Options, PeriodPreset, PeriodSpec, SeriesBy,
    SortOrder, SpecError, Unit, compute,
};
use planning_budget_core::{TxId, YearMonth, load_seed};

const SEED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/seed-2026.json"
));

fn base() -> ChartSpec {
    ChartSpec {
        version: 1,
        title: "Тест".to_owned(),
        chart_type: ChartType::Bar,
        metric: Metric::Spent,
        group_by: GroupBy::Month,
        series_by: None,
        metrics: Vec::new(),
        period: PeriodSpec::Preset {
            preset: PeriodPreset::Ytd,
        },
        filter: String::new(),
        options: Options::default(),
    }
}

fn today() -> YearMonth {
    YearMonth::new(2026, 9).unwrap()
}

fn run(spec: &ChartSpec) -> planning_budget_core::analytics::ChartData {
    let data = load_seed(SEED).unwrap();
    let tags = BTreeMap::new();
    compute(
        &data,
        spec,
        &Context {
            today: today(),
            tx_tags: &tags,
        },
    )
    .unwrap()
}

fn invalid(spec: &ChartSpec) -> SpecError {
    spec.validate().unwrap_err()
}

#[test]
fn json_roundtrip_matches_the_documented_shape() {
    let json = r#"{
        "version": 1, "title": "Траты по статусам", "type": "stacked_bar",
        "metric": "spent", "group_by": "month", "series_by": "status",
        "period": { "preset": "ytd" }, "filter": "тип:желания -кат:займы",
        "options": { "top_n": 10, "sort": "desc" }
    }"#;
    let spec: ChartSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.series_by, Some(SeriesBy::Status));
    assert_eq!(spec.options.sort, SortOrder::Desc);
    assert_eq!(spec.options.top_n, Some(10));
    spec.validate().unwrap();

    let range: PeriodSpec =
        serde_json::from_str(r#"{ "from": "2026-01", "to": "2026-09" }"#).unwrap();
    assert!(matches!(range, PeriodSpec::Range { .. }));
}

#[test]
fn rejects_invalid_combinations_with_a_reason() {
    let mut s = base();
    s.version = 2;
    assert_eq!(invalid(&s), SpecError::Version);

    let mut s = base();
    s.chart_type = ChartType::Donut;
    assert_eq!(invalid(&s), SpecError::Donut);
    s.group_by = GroupBy::Kind;
    s.validate().unwrap();
    s.series_by = Some(SeriesBy::Status);
    assert_eq!(invalid(&s), SpecError::Donut);

    let mut s = base();
    s.metric = Metric::SavingsRate;
    s.group_by = GroupBy::Category;
    assert_eq!(invalid(&s), SpecError::NeedsMonthOrTotal);
    s.group_by = GroupBy::None;
    assert_eq!(invalid(&s), SpecError::SavingsRateByMonth);

    let mut s = base();
    s.metric = Metric::LimitUsage;
    assert_eq!(invalid(&s), SpecError::LimitUsageByCategory);

    let mut s = base();
    s.chart_type = ChartType::StackedBar;
    assert_eq!(invalid(&s), SpecError::StackedWithoutSeries);

    let mut s = base();
    s.group_by = GroupBy::Status;
    s.series_by = Some(SeriesBy::Status);
    assert_eq!(invalid(&s), SpecError::SeriesEqualsGroup);

    let mut s = base();
    s.metric = Metric::Income;
    s.series_by = Some(SeriesBy::Status);
    assert_eq!(invalid(&s), SpecError::SeriesNeedTransactions);

    let mut s = base();
    s.series_by = Some(SeriesBy::Metric);
    s.metrics = vec![Metric::Income];
    assert_eq!(invalid(&s), SpecError::MetricsList);
    s.metrics = vec![Metric::Income, Metric::SavingsRate];
    assert_eq!(invalid(&s), SpecError::MetricsList);

    let mut s = base();
    s.metrics = vec![Metric::Income];
    assert_eq!(invalid(&s), SpecError::MetricsWithoutSeries);

    let mut s = base();
    s.group_by = GroupBy::Category;
    s.options.cumulative = true;
    assert_eq!(invalid(&s), SpecError::Cumulative);
    s.options.cumulative = false;
    s.options.compare_prev_period = true;
    assert_eq!(invalid(&s), SpecError::ComparePrev);

    let mut s = base();
    s.options.show_limit = true;
    assert_eq!(invalid(&s), SpecError::ShowLimit);

    let mut s = base();
    s.options.top_n = Some(0);
    assert_eq!(invalid(&s), SpecError::TopN);

    let mut s = base();
    s.filter = "источник:импорт".to_owned();
    assert_eq!(invalid(&s), SpecError::FilterSource);

    let mut s = base();
    s.period = PeriodSpec::Range {
        from: today(),
        to: YearMonth::new(2026, 1).unwrap(),
    };
    assert_eq!(invalid(&s), SpecError::Period);

    for filter in [
        "статус:абракадабра",
        "сумма>abc",
        "кофе",
        "кат:продукты кофе",
    ] {
        let mut s = base();
        s.filter = filter.to_owned();
        assert_eq!(invalid(&s), SpecError::FilterInvalid, "{filter}");
    }

    let mut s = base();
    s.metric = Metric::AvgTicket;
    s.series_by = Some(SeriesBy::Category);
    s.options.top_n = Some(5);
    assert_eq!(invalid(&s), SpecError::TopNSeries);
    s.metric = Metric::Spent;
    s.validate().unwrap();
}

#[test]
fn filter_narrows_expenses_and_drops_income() {
    let all = run(&base());
    let mut s = base();
    s.filter = "статус:план".to_owned();
    let planned = run(&s);
    let sum = |c: &planning_budget_core::analytics::ChartData| {
        c.series[0].values.iter().flatten().sum::<f64>()
    };
    assert!(sum(&planned) < sum(&all));

    let mut s = base();
    s.metric = Metric::Income;
    s.filter = "кат:продукты".to_owned();
    let income = run(&s);
    assert!(
        income.series[0]
            .values
            .iter()
            .flatten()
            .all(|v| v.abs() < f64::EPSILON)
    );
}

#[test]
fn category_filter_matches_prefix_and_negation_excludes() {
    let mut s = base();
    s.group_by = GroupBy::Category;
    s.filter = "кат:прод".to_owned();
    assert_eq!(run(&s).categories, vec!["Продукты".to_owned()]);

    s.filter = "-кат:продукты".to_owned();
    let c = run(&s);
    assert!(!c.categories.contains(&"Продукты".to_owned()));
    assert!(c.categories.len() > 5);

    s.filter = "кат:нетакой".to_owned();
    assert!(run(&s).categories.is_empty());
}

#[test]
fn top_n_keeps_the_largest_groups_and_merges_series_tail() {
    let mut s = base();
    s.group_by = GroupBy::Category;
    s.options.top_n = Some(3);
    s.options.sort = SortOrder::Desc;
    let c = run(&s);
    assert_eq!(c.categories.len(), 3);
    assert!(c.series[0].values[0] >= c.series[0].values[2]);

    let mut s = base();
    s.chart_type = ChartType::StackedBar;
    s.series_by = Some(SeriesBy::Category);
    s.options.top_n = Some(2);
    let c = run(&s);
    assert_eq!(c.series.len(), 3);
    assert_eq!(c.series.last().unwrap().name, "Прочее");
    let total: f64 = c.totals.as_ref().unwrap().iter().sum();
    let plain = run(&base());
    assert!((total - plain.totals.as_ref().unwrap().iter().sum::<f64>()).abs() < 0.01);
}

#[test]
fn percent_option_sums_columns_to_one_hundred() {
    let mut s = base();
    s.chart_type = ChartType::StackedBar;
    s.series_by = Some(SeriesBy::Kind);
    s.options.percent = true;
    let c = run(&s);
    assert_eq!(c.unit, Unit::Percent);
    assert!(c.totals.is_none());
    for i in 0..c.categories.len() {
        let column: f64 = c.series.iter().filter_map(|s| s.values[i]).sum();
        assert!((column - 100.0).abs() < 0.001, "колонка {i}: {column}");
    }
}

#[test]
fn cumulative_runs_over_months() {
    let plain = run(&base());
    let mut s = base();
    s.options.cumulative = true;
    let cumulative = run(&s);
    let total: f64 = plain.series[0].values.iter().flatten().sum();
    let last = cumulative.series[0]
        .values
        .last()
        .copied()
        .flatten()
        .unwrap();
    assert!((total - last).abs() < 0.01);
}

#[test]
fn compare_previous_period_adds_shifted_series() {
    let mut s = base();
    s.period = PeriodSpec::Range {
        from: YearMonth::new(2026, 4).unwrap(),
        to: today(),
    };
    s.options.compare_prev_period = true;
    let c = run(&s);
    assert_eq!(c.series.len(), 2);
    assert_eq!(c.series[1].name, "Траты (прошлый период)");

    // Апрель–сентябрь сравниваются с октябрём 2025 – мартом 2026: в данных есть только январь–март.
    let year = run(&base());
    assert_eq!(c.series[1].values.len(), 6);
    assert_eq!(
        c.series[1].values.get(3..6),
        year.series[0].values.get(0..3)
    );
    assert_eq!(c.series[1].values.get(0..3), Some(&[Some(0.0); 3][..]));
}

#[test]
fn tags_group_counts_each_tag_and_untagged() {
    let data = load_seed(SEED).unwrap();
    let first = data.transactions.first().unwrap().id;
    let tags: BTreeMap<TxId, Vec<String>> = BTreeMap::from([(first, vec!["подарки".to_owned()])]);
    let mut s = base();
    s.group_by = GroupBy::Tag;
    s.metric = Metric::Count;
    let c = compute(
        &data,
        &s,
        &Context {
            today: today(),
            tx_tags: &tags,
        },
    )
    .unwrap();
    assert_eq!(
        c.categories,
        vec!["Без тега".to_owned(), "#подарки".to_owned()]
    );
    assert_eq!(c.unit, Unit::Count);
    assert!((c.series[0].values[1].unwrap() - 1.0).abs() < f64::EPSILON);

    s.filter = "#подарки".to_owned();
    let only = compute(
        &data,
        &s,
        &Context {
            today: today(),
            tx_tags: &tags,
        },
    )
    .unwrap();
    assert_eq!(only.categories, vec!["#подарки".to_owned()]);
}

#[test]
fn empty_data_gives_zero_series_not_an_error() {
    let mut data = load_seed(SEED).unwrap();
    data.transactions.clear();
    data.incomes.clear();
    let tags = BTreeMap::new();
    let mut s = base();
    s.metric = Metric::SavingsRate;
    let c = compute(
        &data,
        &s,
        &Context {
            today: today(),
            tx_tags: &tags,
        },
    )
    .unwrap();
    assert!(c.series[0].values.iter().all(Option::is_none));
}

#[test]
fn period_presets_resolve_against_today() {
    let data = load_seed(SEED).unwrap();
    let months = |preset| {
        planning_budget_core::analytics::period_months(
            &data,
            PeriodSpec::Preset { preset },
            today(),
        )
        .len()
    };
    assert_eq!(months(PeriodPreset::Ytd), 9);
    assert_eq!(months(PeriodPreset::Last12), 12);
    assert_eq!(months(PeriodPreset::CurrentMonth), 1);
    assert_eq!(months(PeriodPreset::All), 9);
}

#[test]
fn all_time_keeps_the_newest_months_when_history_is_too_long() {
    let mut data = load_seed(SEED).unwrap();
    let mut old = data.incomes.first().unwrap().clone();
    old.month = YearMonth::new(1990, 1).unwrap();
    data.incomes.push(old);
    let months = planning_budget_core::analytics::period_months(
        &data,
        PeriodSpec::Preset {
            preset: PeriodPreset::All,
        },
        today(),
    );
    assert_eq!(months.len(), 240);
    assert_eq!(months.last().copied(), Some(today()));
}

#[test]
fn spec_needs_tags_only_when_the_filter_selects_by_them() {
    let mut s = base();
    assert!(!s.uses_tags(2026));
    s.filter = "кат:продукты сумма>1000".to_owned();
    assert!(!s.uses_tags(2026));
    s.filter = "#подарки".to_owned();
    assert!(s.uses_tags(2026));
    s.filter = "-#подарки".to_owned();
    assert!(s.uses_tags(2026));
}

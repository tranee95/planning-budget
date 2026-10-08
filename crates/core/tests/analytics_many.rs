//! `compute_many`, границы периода и сводка ошибок аналитики.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use planning_budget_core::analytics::{
    AnalyticsError, ChartSpec, Context, PeriodPreset, PeriodSpec, SpecError, compute, compute_many,
    period_bounds, period_months, standard_dashboard,
};
use planning_budget_core::{CoreError, DataSet, MoneyError, YearMonth, load_seed};

const SEED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/seed-2026.json"
));

fn dataset() -> DataSet {
    load_seed(SEED).unwrap()
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

#[test]
fn compute_many_equals_single_compute_for_every_standard_chart() {
    let data = dataset();
    let tags = BTreeMap::new();
    let ctx = Context {
        today: ym(2026, 9),
        tx_tags: &tags,
    };
    let placements = standard_dashboard();
    let specs: Vec<&ChartSpec> = placements.iter().map(|p| &p.spec).collect();

    let many = compute_many(&data, &specs, &ctx).unwrap();

    assert_eq!(many.len(), specs.len());
    for (spec, got) in specs.iter().zip(many) {
        assert_eq!(got.unwrap(), compute(&data, spec, &ctx).unwrap());
    }
}

#[test]
fn a_broken_spec_does_not_spoil_the_other_charts() {
    let data = dataset();
    let tags = BTreeMap::new();
    let ctx = Context {
        today: ym(2026, 9),
        tx_tags: &tags,
    };
    let placements = standard_dashboard();
    let mut broken = placements[0].spec.clone();
    broken.version = 2;
    let specs = [&placements[0].spec, &broken, &placements[1].spec];

    let many = compute_many(&data, &specs, &ctx).unwrap();

    assert!(many[0].is_ok());
    assert_eq!(many[1], Err(AnalyticsError::Spec(SpecError::Version)));
    assert!(many[2].is_ok());
}

#[test]
fn compute_many_of_nothing_is_empty() {
    let data = dataset();
    let tags = BTreeMap::new();
    let ctx = Context {
        today: ym(2026, 9),
        tx_tags: &tags,
    };
    assert!(compute_many(&data, &[], &ctx).unwrap().is_empty());
}

#[test]
fn period_bounds_by_preset_and_range() {
    let today = ym(2026, 9);
    let preset = |preset| PeriodSpec::Preset { preset };
    assert_eq!(
        period_bounds(preset(PeriodPreset::Ytd), today),
        Some((ym(2026, 1), today))
    );
    assert_eq!(
        period_bounds(preset(PeriodPreset::Last12), today),
        Some((ym(2025, 10), today))
    );
    assert_eq!(
        period_bounds(preset(PeriodPreset::CurrentMonth), today),
        Some((today, today))
    );
    assert_eq!(period_bounds(preset(PeriodPreset::All), today), None);
    let range = PeriodSpec::Range {
        from: ym(2026, 2),
        to: ym(2026, 4),
    };
    assert_eq!(
        period_bounds(range, today),
        Some((ym(2026, 2), ym(2026, 4)))
    );
}

#[test]
fn period_months_lists_every_month_of_the_period() {
    let data = dataset();
    let today = ym(2026, 9);
    let range = PeriodSpec::Range {
        from: ym(2026, 2),
        to: ym(2026, 4),
    };
    assert_eq!(
        period_months(&data, range, today),
        vec![ym(2026, 2), ym(2026, 3), ym(2026, 4)]
    );
    let current = PeriodSpec::Preset {
        preset: PeriodPreset::CurrentMonth,
    };
    assert_eq!(period_months(&data, current, today), vec![today]);
    let all = period_months(
        &data,
        PeriodSpec::Preset {
            preset: PeriodPreset::All,
        },
        today,
    );
    assert!(!all.is_empty(), "the period always has at least one month");
}

#[test]
fn money_errors_become_core_errors_of_analytics() {
    let err = AnalyticsError::from(MoneyError::Overflow);
    assert_eq!(
        err,
        AnalyticsError::Core(CoreError::from(MoneyError::Overflow))
    );
}

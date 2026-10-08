//! Golden-тесты расчёты на `seed-2026.json` против `golden-2026.json`.
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used
)]

use planning_budget_core::calc::{Ledger, Scenario};
use planning_budget_core::{CategoryKind, DataSet, Money, TxStatus, YearMonth, load_seed};
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
const RATIO_TOL: f64 = 1e-4;
const YEAR: u16 = 2026;

fn dataset() -> DataSet {
    load_seed(SEED).unwrap()
}

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn current() -> YearMonth {
    YearMonth::new(YEAR, 9).unwrap()
}

fn num(v: &Value, ctx: &str) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| panic!("golden has no number at {ctx}"))
}

fn check_rub(errs: &mut Vec<String>, ctx: &str, actual: Money, expected: &Value, tol: f64) {
    let expected = num(expected, ctx);
    let actual_rub = actual.as_f64() / 100.0;
    if (actual_rub - expected).abs() > tol {
        errs.push(format!("{ctx}: {actual_rub:.4} ≠ {expected:.4}"));
    }
}

fn check_ratio(errs: &mut Vec<String>, ctx: &str, actual: Option<f64>, expected: &Value) {
    let expected = num(expected, ctx);
    match actual {
        Some(a) if (a - expected).abs() <= RATIO_TOL => {}
        other => errs.push(format!("{ctx}: {other:?} ≠ {expected:.6}")),
    }
}

fn finish(errs: &[String]) {
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

/// Месяцы golden: ключ — русское имя месяца 2026 года.
fn golden_months(g: &Value) -> Vec<(YearMonth, &Value)> {
    YearMonth::months_of_year(YEAR)
        .unwrap()
        .filter_map(|m| g["months"].get(m.month_name_ru()).map(|v| (m, v)))
        .collect()
}

#[test]
fn golden_covers_january_to_september() {
    assert_eq!(golden_months(&golden()).len(), 9);
}

#[test]
fn months_summary() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let g = golden();
    let mut errs = Vec::new();
    for (month, expected) in golden_months(&g) {
        let s = ledger.month_summary(month).unwrap();
        let name = month.month_name_ru();
        check_rub(
            &mut errs,
            &format!("{name}.income"),
            s.income,
            &expected["income"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{name}.expenses"),
            s.expenses,
            &expected["expenses"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{name}.savings"),
            s.savings,
            &expected["savings"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{name}.free"),
            s.free,
            &expected["free"],
            RUB_TOL,
        );
        check_ratio(
            &mut errs,
            &format!("{name}.savings_rate"),
            s.savings_rate,
            &expected["savings_rate"],
        );
    }
    finish(&errs);
}

#[test]
fn status_and_kind_totals() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let g = golden();
    let statuses = [
        ("paid", TxStatus::Paid),
        ("debt", TxStatus::Debt),
        ("unplanned", TxStatus::Unplanned),
        ("planned", TxStatus::Planned),
    ];
    let kinds = [
        ("mandatory", CategoryKind::Mandatory),
        ("wants", CategoryKind::Wants),
        ("savings", CategoryKind::Savings),
        ("loans", CategoryKind::Loans),
    ];
    let mut errs = Vec::new();
    for (month, expected) in golden_months(&g) {
        let s = ledger.month_summary(month).unwrap();
        let name = month.month_name_ru();
        for (key, status) in statuses {
            check_rub(
                &mut errs,
                &format!("{name}.status.{key}"),
                s.by_status.get(status),
                &expected["status"][key],
                RUB_TOL,
            );
        }
        for (key, kind) in kinds {
            check_rub(
                &mut errs,
                &format!("{name}.kinds.{key}"),
                s.by_kind.get(kind),
                &expected["kinds"][key],
                RUB_TOL,
            );
        }
    }
    finish(&errs);
}

#[test]
fn categories() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let g = golden();
    let mut errs = Vec::new();
    for (month, expected) in golden_months(&g) {
        let Some(cats) = expected["categories"].as_object() else {
            errs.push(format!(
                "{}: no categories in golden",
                month.month_name_ru()
            ));
            continue;
        };
        for (cat_name, value) in cats {
            let Some(category) = data.categories.iter().find(|c| &c.name == cat_name) else {
                errs.push(format!("unknown category {cat_name}"));
                continue;
            };
            let actual = ledger.category_total(month, category.id).unwrap();
            check_rub(
                &mut errs,
                &format!("{}.{cat_name}", month.month_name_ru()),
                actual,
                value,
                RUB_TOL,
            );
        }
    }
    finish(&errs);
}

#[test]
fn year_totals() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let g = golden();
    let y = ledger.year_summary(YEAR, current()).unwrap();
    let mut errs = Vec::new();
    check_rub(
        &mut errs,
        "year.income",
        y.income,
        &g["year"]["income"],
        RUB_TOL,
    );
    check_rub(
        &mut errs,
        "year.expenses",
        y.expenses,
        &g["year"]["expenses"],
        RUB_TOL,
    );
    check_rub(
        &mut errs,
        "year.savings",
        y.savings,
        &g["year"]["savings"],
        RUB_TOL,
    );
    check_rub(&mut errs, "year.free", y.free, &g["year"]["free"], RUB_TOL);
    check_rub(
        &mut errs,
        "year.avg_income",
        y.avg_income,
        &g["year"]["avg_income"],
        RUB_TOL,
    );
    check_ratio(
        &mut errs,
        "year.savings_rate",
        y.savings_rate,
        &g["year"]["savings_rate"],
    );
    if Some(u64::from(y.months_with_data)) != g["year"]["months_with_data"].as_u64() {
        errs.push(format!("year.months_with_data: {}", y.months_with_data));
    }
    finish(&errs);
}

/// Единственная категория-сбережение эталона: раздел «Облигации» таблицы — её накопление.
fn sole_savings(data: &DataSet) -> planning_budget_core::CategoryId {
    let ids: Vec<_> = data
        .categories
        .iter()
        .filter(|c| c.kind == CategoryKind::Savings)
        .map(|c| c.id)
        .collect();
    assert_eq!(ids.len(), 1, "в эталоне одна категория-сбережение");
    ids[0]
}

#[test]
fn bonds() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let savings = sole_savings(&data);
    let g = golden();
    let mut errs = Vec::new();

    let fact = ledger.accumulation_fact(savings, YEAR).unwrap();
    for (month, expected) in golden_months(&g) {
        let Some(row) = fact.months.iter().find(|r| r.month == month) else {
            errs.push(format!("{}: no bonds row", month.month_name_ru()));
            continue;
        };
        let name = month.month_name_ru();
        check_rub(
            &mut errs,
            &format!("{name}.bonds_end_balance"),
            row.balance,
            &expected["bonds_end_balance"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{name}.bonds_coupon"),
            row.coupon,
            &expected["bonds_coupon"],
            RUB_TOL,
        );
    }
    check_rub(
        &mut errs,
        "bonds.dec_balance",
        fact.dec_balance,
        &g["bonds"]["dec_balance"],
        RUB_TOL,
    );

    let params = data.savings_params[&savings];
    let rates = planning_budget_core::calc::BondRates::new(params.annual_rate, params.tax);
    check_ratio(
        &mut errs,
        "bonds.effective_rate",
        Some(rates.effective),
        &g["bonds"]["effective_rate"],
    );
    check_ratio(
        &mut errs,
        "bonds.monthly_rate",
        Some(rates.monthly),
        &g["bonds"]["monthly_rate"],
    );

    let forecast = ledger
        .accumulation_forecast(savings, YEAR, current())
        .unwrap();
    for s in forecast.scenarios {
        let key = match s.scenario {
            Scenario::A => "A",
            Scenario::B => "B",
            Scenario::C => "C",
        };
        let expected = &g["bonds"]["scenarios"][key];
        check_rub(
            &mut errs,
            &format!("{key}.contribution"),
            s.contribution,
            &expected["contribution"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{key}.y1"),
            s.y1,
            &expected["y1"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{key}.y3"),
            s.y3,
            &expected["y3"],
            RUB_TOL,
        );
        check_rub(
            &mut errs,
            &format!("{key}.y5"),
            s.y5,
            &expected["y5"],
            RUB_TOL,
        );
    }
    finish(&errs);
}

#[test]
fn reference_balance() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let g = golden();
    let b = ledger.budget_balance(YEAR, current()).unwrap();
    let r = &g["reference"];
    let mut errs = Vec::new();
    check_rub(
        &mut errs,
        "avg_income",
        b.avg_income,
        &r["Средний доход"],
        RUB_TOL,
    );
    check_rub(
        &mut errs,
        "savings_target",
        b.savings_target,
        &r["Сбережения по цели (норма)"],
        RUB_TOL,
    );
    check_rub(
        &mut errs,
        "limits_sum",
        b.limits_sum,
        &r["Лимиты расходов"],
        RUB_TOL,
    );
    check_rub(
        &mut errs,
        "buffer",
        b.buffer,
        &r["Буфер: не распределено"],
        RUB_TOL,
    );
    check_ratio(
        &mut errs,
        "buffer_rate",
        b.buffer_rate,
        &r["Буфер, % дохода"],
    );
    check_rub(
        &mut errs,
        "economy",
        b.economy,
        &r["Экономия от лимитов vs факт"],
        RUB_TOL,
    );
    finish(&errs);
}

#[test]
fn forecast_lines_pass_through_the_one_three_and_five_year_totals() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let forecast = ledger
        .accumulation_forecast(sole_savings(&data), YEAR, current())
        .unwrap();
    for s in &forecast.scenarios {
        assert_eq!(s.balances.len(), 60);
        assert_eq!(s.balances[11], s.y1);
        assert_eq!(s.balances[35], s.y3);
        assert_eq!(s.balances[59], s.y5);
        assert!(
            s.balances.windows(2).all(|w| w[0] < w[1]),
            "баланс растёт каждый месяц"
        );
    }
}

/// Раздел «Сбережения» с единственным накоплением даёт те же итоги, что само накопление.
#[test]
fn overview_of_a_single_accumulation_equals_the_accumulation() {
    let data = dataset();
    let ledger = Ledger::new(&data).unwrap();
    let id = sole_savings(&data);
    let fact = ledger.accumulation_fact(id, YEAR).unwrap();
    // Раздел считает планы и средние на декабрь года просмотра.
    let december = YearMonth::new(YEAR, 12).unwrap();
    let forecast = ledger.accumulation_forecast(id, YEAR, december).unwrap();

    let overview = ledger.savings_overview(YEAR).unwrap();
    assert_eq!(overview.items.len(), 1);
    assert_eq!(overview.items[0].fact, fact);
    assert_eq!(overview.items[0].forecast, forecast);
    assert_eq!(overview.total_balance, fact.dec_balance);
    for (total, scenario) in overview.total_forecast.iter().zip(&forecast.scenarios) {
        assert_eq!(total, scenario);
    }
}

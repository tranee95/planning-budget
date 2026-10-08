//! Данные графиков поверх `core::analytics`.

use chrono::{DateTime, NaiveDate, Utc};
use planning_budget_core::YearMonth;
use planning_budget_core::analytics::{
    AnalyticsError, ChartData, ChartSpec, Context, Options, PeriodSpec, SpecError, Unit, compute,
    compute_many, period_bounds,
};
use planning_budget_storage::{CardPlacement, ChartCard, Dashboard, Db};
use std::collections::BTreeMap;

use super::{month_of, parse_month};
use crate::AppError;
use crate::dto::{
    CardPlacementDto, ChartCardDto, ChartDataDto, ChartOptionsDto, ChartPeriodDto, ChartRefLineDto,
    ChartRunDto, ChartSeriesDto, ChartSpecDto, ChartUnitDto, DashboardDto,
};

pub(crate) fn spec_from_dto(dto: &ChartSpecDto) -> Result<ChartSpec, AppError> {
    let period = match &dto.period {
        ChartPeriodDto::Preset { preset } => PeriodSpec::Preset {
            preset: (*preset).into(),
        },
        ChartPeriodDto::Range { from, to } => PeriodSpec::Range {
            from: parse_month(from, "period")?,
            to: parse_month(to, "period")?,
        },
    };
    let options = &dto.options;
    Ok(ChartSpec {
        version: dto.version,
        title: dto.title.clone(),
        chart_type: dto.chart_type.into(),
        metric: dto.metric.into(),
        group_by: dto.group_by.into(),
        series_by: dto.series_by.map(Into::into),
        metrics: dto.metrics.iter().copied().map(Into::into).collect(),
        period,
        filter: dto.filter.clone(),
        options: Options {
            show_limit: options.show_limit,
            top_n: options.top_n,
            sort: options.sort.into(),
            cumulative: options.cumulative,
            compare_prev_period: options.compare_prev_period,
            percent: options.percent,
        },
    })
}

pub(crate) fn spec_to_dto(spec: &ChartSpec) -> Result<ChartSpecDto, AppError> {
    let period = match spec.period {
        PeriodSpec::Preset { preset } => ChartPeriodDto::Preset {
            preset: preset.into(),
        },
        PeriodSpec::Range { from, to } => ChartPeriodDto::Range {
            from: from.to_string(),
            to: to.to_string(),
        },
    };
    let options = &spec.options;
    Ok(ChartSpecDto {
        version: spec.version,
        title: spec.title.clone(),
        chart_type: spec.chart_type.into(),
        metric: spec.metric.into(),
        group_by: spec.group_by.into(),
        series_by: spec.series_by.map(Into::into),
        metrics: spec.metrics.iter().copied().map(Into::into).collect(),
        period,
        filter: spec.filter.clone(),
        options: ChartOptionsDto {
            show_limit: options.show_limit,
            top_n: options.top_n,
            sort: options.sort.into(),
            cumulative: options.cumulative,
            compare_prev_period: options.compare_prev_period,
            percent: options.percent,
        },
    })
}

impl From<ChartData> for ChartDataDto {
    fn from(data: ChartData) -> Self {
        Self {
            categories: data.categories,
            category_tokens: data.category_tokens,
            series: data
                .series
                .into_iter()
                .map(|s| ChartSeriesDto {
                    name: s.name,
                    color: s.color,
                    values: s.values,
                })
                .collect(),
            reference_lines: data
                .reference_lines
                .into_iter()
                .map(|l| ChartRefLineDto {
                    name: l.name,
                    from: l.from,
                    to: l.to,
                })
                .collect(),
            totals: data.totals,
            unit: match data.unit {
                Unit::Rub => ChartUnitDto::Rub,
                Unit::Percent => ChartUnitDto::Percent,
                Unit::Count => ChartUnitDto::Count,
            },
        }
    }
}

fn spec_error(e: SpecError) -> AppError {
    AppError::Validation {
        message_key: format!("errors.{}", e.key()),
        field: None,
    }
}

/// Сколько вариантов описания конструктор проверяет за один вызов.
const MAX_CHECKED: usize = 128;

/// Ключ причины, по которой описание нельзя построить, для каждого варианта; `None` — можно.
/// Данные не читаются: конструктор так отключает недопустимые значения вместе с причиной.
pub fn check(specs: &[ChartSpecDto]) -> Result<Vec<Option<String>>, AppError> {
    if specs.len() > MAX_CHECKED {
        return Err(AppError::invalid_field("chart.too_many_checks", "specs"));
    }
    Ok(specs
        .iter()
        .map(|dto| match spec_from_dto(dto) {
            Ok(spec) => spec.validate().err().map(|e| format!("errors.{}", e.key())),
            Err(AppError::Validation { message_key, .. }) => Some(message_key),
            Err(_) => Some("errors.chart.invalid".to_owned()),
        })
        .collect())
}

/// Считает график. Загружаются только месяцы периода (и предыдущего, если он нужен для
/// сравнения); `all` читает всю историю.
pub fn run(db: &Db, spec: &ChartSpecDto, today: NaiveDate) -> Result<ChartDataDto, AppError> {
    let spec = spec_from_dto(spec)?;
    spec.validate().map_err(spec_error)?;
    let today = month_of(today)?;
    let (from, to) = load_bounds(&spec, today)?;
    let data = db.dataset(from, to)?;
    let tx_tags = if spec.uses_tags(today.year()) {
        db.tx_tag_names(from, to)?
    } else {
        BTreeMap::new()
    };
    let ctx = Context {
        today,
        tx_tags: &tx_tags,
    };
    let chart = compute(&data, &spec, &ctx).map_err(analytics_error)?;
    Ok(chart.into())
}

/// Месяцы, которые нужно загрузить для графика: период и предыдущий, если он нужен для сравнения.
fn load_bounds(spec: &ChartSpec, today: YearMonth) -> Result<(YearMonth, YearMonth), AppError> {
    Ok(match period_bounds(spec.period, today) {
        Some((from, to)) if spec.options.compare_prev_period => {
            let months = from.iter_to(to).count();
            let back = (0..months).try_fold(from, |m, _| m.pred()).unwrap_or(from);
            (back, to)
        }
        Some(bounds) => bounds,
        None => (YearMonth::new(1970, 1)?, YearMonth::new(9999, 12)?),
    })
}

/// Сколько графиков считается за один пакетный вызов (дашборд — единицы, предел против мусора).
const MAX_RUN_MANY: usize = 64;

/// Считает несколько графиков одним набором данных на самый широкий период и одним `Ledger`.
/// Результат каждого графика равен `run`; недопустимое описание даёт ключ причины у своего
/// графика, остальные считаются. Ошибки хранилища и расчёта по-прежнему отменяют весь вызов.
pub fn run_many(
    db: &Db,
    specs: &[ChartSpecDto],
    today: NaiveDate,
) -> Result<Vec<ChartRunDto>, AppError> {
    if specs.len() > MAX_RUN_MANY {
        return Err(AppError::invalid_field("chart.too_many_specs", "specs"));
    }
    let today = month_of(today)?;
    let mut out: Vec<ChartRunDto> = specs
        .iter()
        .map(|_| ChartRunDto {
            data: None,
            error_key: None,
        })
        .collect();
    let mut valid: Vec<(usize, ChartSpec)> = Vec::new();
    for (i, dto) in specs.iter().enumerate() {
        match spec_from_dto(dto).and_then(|spec| spec.validate().map_err(spec_error).map(|()| spec))
        {
            Ok(spec) => valid.push((i, spec)),
            Err(AppError::Validation { message_key, .. }) => {
                if let Some(slot) = out.get_mut(i) {
                    slot.error_key = Some(message_key);
                }
            }
            Err(e) => return Err(e),
        }
    }
    if valid.is_empty() {
        return Ok(out);
    }

    let mut from = YearMonth::new(9999, 12)?;
    let mut to = YearMonth::new(1970, 1)?;
    for (_, spec) in &valid {
        let (f, t) = load_bounds(spec, today)?;
        from = from.min(f);
        to = to.max(t);
    }
    let data = db.dataset(from, to)?;
    let tx_tags = if valid.iter().any(|(_, s)| s.uses_tags(today.year())) {
        db.tx_tag_names(from, to)?
    } else {
        BTreeMap::new()
    };
    let ctx = Context {
        today,
        tx_tags: &tx_tags,
    };
    let refs: Vec<&ChartSpec> = valid.iter().map(|(_, s)| s).collect();
    let results = compute_many(&data, &refs, &ctx).map_err(analytics_error)?;
    for ((i, _), result) in valid.iter().zip(results) {
        let Some(slot) = out.get_mut(*i) else {
            continue;
        };
        match result {
            Ok(chart) => slot.data = Some(chart.into()),
            Err(AnalyticsError::Spec(e)) => {
                if let AppError::Validation { message_key, .. } = spec_error(e) {
                    slot.error_key = Some(message_key);
                }
            }
            Err(AnalyticsError::Core(e)) => return Err(e.into()),
        }
    }
    Ok(out)
}

fn analytics_error(e: AnalyticsError) -> AppError {
    match e {
        AnalyticsError::Spec(e) => spec_error(e),
        AnalyticsError::Core(e) => e.into(),
    }
}

fn card_dto(card: ChartCard) -> Result<ChartCardDto, AppError> {
    Ok(ChartCardDto {
        id: card.id,
        dashboard_id: card.dashboard_id,
        spec: spec_to_dto(&card.spec)?,
        x: card.x,
        y: card.y,
        w: card.w,
        h: card.h,
    })
}

pub fn dashboards_list(db: &Db) -> Result<Vec<DashboardDto>, AppError> {
    Ok(db.dashboards()?.into_iter().map(dashboard_dto).collect())
}

fn dashboard_dto(d: Dashboard) -> DashboardDto {
    DashboardDto {
        id: d.id,
        name: d.name,
        is_default: d.is_default,
    }
}

pub fn dashboard_create(db: &mut Db, name: &str) -> Result<DashboardDto, AppError> {
    Ok(dashboard_dto(db.dashboard_create(name)?))
}

pub fn dashboard_create_default(db: &mut Db, now: DateTime<Utc>) -> Result<DashboardDto, AppError> {
    db.seed_standard_dashboard(now)?;
    db.dashboards()?
        .into_iter()
        .next()
        .map(dashboard_dto)
        .ok_or_else(|| AppError::Conflict {
            message_key: "errors.dashboard.exists".into(),
        })
}

pub fn dashboard_rename(db: &mut Db, id: i64, name: &str) -> Result<(), AppError> {
    Ok(db.dashboard_rename(id, name)?)
}

pub fn dashboard_delete(db: &mut Db, id: i64) -> Result<(), AppError> {
    Ok(db.dashboard_delete(id)?)
}

pub fn charts_list(db: &Db, dashboard_id: i64) -> Result<Vec<ChartCardDto>, AppError> {
    db.charts(dashboard_id)?.into_iter().map(card_dto).collect()
}

pub fn chart_create(
    db: &mut Db,
    dashboard_id: i64,
    spec: &ChartSpecDto,
    w: u8,
    h: u8,
    now: DateTime<Utc>,
) -> Result<ChartCardDto, AppError> {
    card_dto(db.chart_create(dashboard_id, &spec_from_dto(spec)?, w, h, now)?)
}

pub fn chart_update(
    db: &mut Db,
    id: i64,
    spec: &ChartSpecDto,
    now: DateTime<Utc>,
) -> Result<ChartCardDto, AppError> {
    card_dto(db.chart_update(id, &spec_from_dto(spec)?, now)?)
}

pub fn chart_delete(db: &mut Db, id: i64) -> Result<(), AppError> {
    Ok(db.chart_delete(id)?)
}

pub fn charts_layout_set(
    db: &mut Db,
    dashboard_id: i64,
    placements: &[CardPlacementDto],
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let placements: Vec<CardPlacement> = placements
        .iter()
        .map(|p| CardPlacement {
            id: p.id.0,
            x: p.x,
            y: p.y,
            w: p.w,
            h: p.h,
        })
        .collect();
    Ok(db.charts_layout_set(dashboard_id, &placements, now)?)
}

#[cfg(test)]
mod tests {
    use planning_budget_core::analytics::standard_dashboard;

    use super::*;

    #[test]
    fn check_reports_a_reason_per_variant() {
        let mut ok = spec_to_dto(&standard_dashboard().swap_remove(0).spec).unwrap();
        let mut bad = ok.clone();
        bad.chart_type = crate::dto::ChartTypeDto::Donut;
        bad.series_by = None;
        bad.metrics.clear();
        ok.title = "Другое".to_owned();
        let reasons = check(&[ok, bad]).unwrap();
        assert_eq!(reasons[0], None);
        assert_eq!(reasons[1].as_deref(), Some("errors.chart.donut"));
        assert!(
            check(&vec![
                spec_to_dto(&standard_dashboard().swap_remove(0).spec)
                    .unwrap();
                129
            ])
            .is_err()
        );
    }

    #[test]
    fn standard_specs_survive_the_dto_round_trip() {
        for placement in standard_dashboard() {
            let dto = spec_to_dto(&placement.spec).unwrap();
            assert_eq!(spec_from_dto(&dto).unwrap(), placement.spec);
        }
    }
}

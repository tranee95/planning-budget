//! Данные графиков поверх `core::analytics`.

use budget_core::YearMonth;
use budget_core::analytics::{
    AnalyticsError, ChartData, ChartSpec, Context, Options, PeriodSpec, SpecError, Unit, compute,
    period_bounds,
};
use budget_storage::{CardPlacement, ChartCard, Dashboard, Db};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

use super::{month_of, parse_month};
use crate::AppError;
use crate::dto::{
    CardPlacementDto, ChartCardDto, ChartDataDto, ChartOptionsDto, ChartPeriodDto, ChartRefLineDto,
    ChartSeriesDto, ChartSpecDto, ChartUnitDto, DashboardDto,
};

/// Перечисления DTO и ядра совпадают по именам значений; тест проверяет каждое.
fn same<A: Serialize, B: DeserializeOwned>(value: &A) -> Result<B, AppError> {
    serde_json::to_value(value)
        .and_then(serde_json::from_value)
        .map_err(|e| AppError::internal("chart_spec", &e))
}

pub(crate) fn spec_from_dto(dto: &ChartSpecDto) -> Result<ChartSpec, AppError> {
    let period = match &dto.period {
        ChartPeriodDto::Preset { preset } => PeriodSpec::Preset {
            preset: same(preset)?,
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
        chart_type: same(&dto.chart_type)?,
        metric: same(&dto.metric)?,
        group_by: same(&dto.group_by)?,
        series_by: dto.series_by.as_ref().map(same).transpose()?,
        metrics: dto.metrics.iter().map(same).collect::<Result<_, _>>()?,
        period,
        filter: dto.filter.clone(),
        options: Options {
            show_limit: options.show_limit,
            top_n: options.top_n,
            sort: same(&options.sort)?,
            cumulative: options.cumulative,
            compare_prev_period: options.compare_prev_period,
            percent: options.percent,
        },
    })
}

pub(crate) fn spec_to_dto(spec: &ChartSpec) -> Result<ChartSpecDto, AppError> {
    let period = match spec.period {
        PeriodSpec::Preset { preset } => ChartPeriodDto::Preset {
            preset: same(&preset)?,
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
        chart_type: same(&spec.chart_type)?,
        metric: same(&spec.metric)?,
        group_by: same(&spec.group_by)?,
        series_by: spec.series_by.as_ref().map(same).transpose()?,
        metrics: spec.metrics.iter().map(same).collect::<Result<_, _>>()?,
        period,
        filter: spec.filter.clone(),
        options: ChartOptionsDto {
            show_limit: options.show_limit,
            top_n: options.top_n,
            sort: same(&options.sort)?,
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
    let (from, to) = match period_bounds(spec.period, today) {
        Some((from, to)) if spec.options.compare_prev_period => {
            let months = from.iter_to(to).count();
            let back = (0..months).try_fold(from, |m, _| m.pred()).unwrap_or(from);
            (back, to)
        }
        Some(bounds) => bounds,
        None => (YearMonth::new(1970, 1)?, YearMonth::new(9999, 12)?),
    };
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
    let chart = compute(&data, &spec, &ctx).map_err(|e| match e {
        AnalyticsError::Spec(e) => spec_error(e),
        AnalyticsError::Core(e) => e.into(),
    })?;
    Ok(chart.into())
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
    use budget_core::analytics::standard_dashboard;

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

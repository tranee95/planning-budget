//! Данные для графиков: `ChartSpec` → `ChartData`. Чистые расчёты над `DataSet`,
//! цвета серий — токены, а не hex.

mod engine;
mod matcher;
mod spec;
mod standard;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use spec::{
    ChartSpec, ChartType, GroupBy, Metric, Options, PeriodPreset, PeriodSpec, SeriesBy, SortOrder,
    SpecError, Unit,
};

pub use standard::{Placement, standard_dashboard};

use crate::calc::Ledger;
use crate::error::CoreError;
use crate::model::{DataSet, TxId};
use crate::period::YearMonth;

/// Внешние условия расчёта: `core` не знает ни даты, ни тегов.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a> {
    /// Текущий месяц: от него считаются пресеты периода.
    pub today: YearMonth,
    /// Имена тегов по id траты; траты без записи считаются без тегов.
    pub tx_tags: &'a BTreeMap<TxId, Vec<String>>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Series {
    pub name: String,
    /// Токен цвета: `status.paid`, `kind.wants`, `category:<id>`, `metric.income`, `other`.
    pub color: String,
    /// По одному значению на подпись оси; `None` — нет данных.
    pub values: Vec<Option<f64>>,
}

/// Опорная линия или полоса (`to` заполнено) в единицах графика.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RefLine {
    pub name: String,
    pub from: f64,
    pub to: Option<f64>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ChartData {
    /// Подписи оси X или секторов.
    pub categories: Vec<String>,
    /// Токен каждой подписи: цвет у статусов, типов и категорий (`status.paid`, `kind.wants`,
    /// `category:<id>`), `month:YYYY-MM` у месяцев (для перехода к записям), иначе пустая строка.
    pub category_tokens: Vec<String>,
    pub series: Vec<Series>,
    pub reference_lines: Vec<RefLine>,
    /// Сумма по подписи; только для суммируемых показателей.
    pub totals: Option<Vec<f64>>,
    pub unit: Unit,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnalyticsError {
    #[error(transparent)]
    Spec(#[from] SpecError),
    #[error(transparent)]
    Core(#[from] CoreError),
}

impl From<crate::MoneyError> for AnalyticsError {
    fn from(err: crate::MoneyError) -> Self {
        Self::Core(err.into())
    }
}

/// Проверяет описание и считает данные графика.
pub fn compute(
    data: &DataSet,
    spec: &ChartSpec,
    ctx: &Context<'_>,
) -> Result<ChartData, AnalyticsError> {
    spec.validate()?;
    let ledger = Ledger::new(data)?;
    engine::Engine::new(data, spec, ctx, &ledger)?.run()
}

/// Считает несколько графиков над одним набором данных и одним `Ledger`. Набор может быть шире
/// периода любого графика: результат равен одиночному `compute` на наборе, покрывающем период
/// графика. Ошибка описания или расчёта одного графика не мешает остальным; ошибка самого
/// `Ledger` (общая для всех) возвращается сразу.
pub fn compute_many(
    data: &DataSet,
    specs: &[&ChartSpec],
    ctx: &Context<'_>,
) -> Result<Vec<Result<ChartData, AnalyticsError>>, AnalyticsError> {
    let ledger = Ledger::new(data)?;
    Ok(specs
        .iter()
        .map(|spec| {
            spec.validate()?;
            engine::Engine::new(data, spec, ctx, &ledger)?.run()
        })
        .collect())
}

/// Месяцы периода описания. Пустого периода не бывает: всегда есть хотя бы один месяц.
pub fn period_months(data: &DataSet, period: PeriodSpec, today: YearMonth) -> Vec<YearMonth> {
    engine::period_months(data, period, today)
}

/// Границы периода без чтения данных (чтобы загрузить нужные месяцы); `None` у пресета `all`.
pub fn period_bounds(period: PeriodSpec, today: YearMonth) -> Option<(YearMonth, YearMonth)> {
    engine::period_bounds(period, today)
}

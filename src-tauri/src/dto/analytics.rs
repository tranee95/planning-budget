//! Графики и дашборды.

use planning_budget_core::analytics::{
    ChartType, GroupBy, Metric, PeriodPreset, SeriesBy, SortOrder,
};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::Int53;

/// `From` в обе стороны между перечислением DTO и перечислением ядра с одинаковыми вариантами.
/// Новый вариант в одном из них ломает сборку, а не обнаруживается на выполнении.
macro_rules! enum_pair {
    ($dto:ident <-> $core:ident { $($variant:ident),+ $(,)? }) => {
        impl From<$core> for $dto {
            fn from(value: $core) -> Self {
                match value { $($core::$variant => Self::$variant),+ }
            }
        }
        impl From<$dto> for $core {
            fn from(value: $dto) -> Self {
                match value { $($dto::$variant => Self::$variant),+ }
            }
        }
    };
}

enum_pair!(ChartTypeDto <-> ChartType { Bar, StackedBar, Hbar, Line, Area, Donut, Table, Kpi });
enum_pair!(ChartMetricDto <-> Metric {
    Spent, Expenses, Income, Savings, Free, FreeCum, SavingsCum, SavingsRate, LimitUsage, Count, AvgTicket
});
enum_pair!(ChartGroupByDto <-> GroupBy { Month, Category, Kind, Status, Tag, None });
enum_pair!(ChartSeriesByDto <-> SeriesBy { Status, Kind, Category, Metric });
enum_pair!(PeriodPresetDto <-> PeriodPreset { Ytd, Last12, CurrentMonth, All });
enum_pair!(ChartSortDto <-> SortOrder { Desc, Asc, Natural });

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartTypeDto {
    Bar,
    StackedBar,
    Hbar,
    Line,
    Area,
    Donut,
    Table,
    Kpi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartMetricDto {
    Spent,
    Expenses,
    Income,
    Savings,
    Free,
    FreeCum,
    SavingsCum,
    SavingsRate,
    LimitUsage,
    Count,
    AvgTicket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartGroupByDto {
    Month,
    Category,
    Kind,
    Status,
    Tag,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartSeriesByDto {
    Status,
    Kind,
    Category,
    Metric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PeriodPresetDto {
    Ytd,
    Last12,
    CurrentMonth,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartSortDto {
    Desc,
    Asc,
    Natural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ChartUnitDto {
    Rub,
    Percent,
    Count,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum ChartPeriodDto {
    Preset { preset: PeriodPresetDto },
    Range { from: String, to: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartOptionsDto {
    pub show_limit: bool,
    pub top_n: Option<u32>,
    pub sort: ChartSortDto,
    pub cumulative: bool,
    pub compare_prev_period: bool,
    pub percent: bool,
}

/// `ChartSpec` v1; в базе лежит JSON `planning_budget_core::analytics::ChartSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartSpecDto {
    pub version: u8,
    pub title: String,
    #[serde(rename = "type")]
    pub chart_type: ChartTypeDto,
    pub metric: ChartMetricDto,
    pub group_by: ChartGroupByDto,
    pub series_by: Option<ChartSeriesByDto>,
    pub metrics: Vec<ChartMetricDto>,
    pub period: ChartPeriodDto,
    pub filter: String,
    pub options: ChartOptionsDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeriesDto {
    pub name: String,
    pub color: String,
    pub values: Vec<Option<f64>>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartRefLineDto {
    pub name: String,
    pub from: f64,
    pub to: Option<f64>,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartDataDto {
    pub categories: Vec<String>,
    pub category_tokens: Vec<String>,
    pub series: Vec<ChartSeriesDto>,
    pub reference_lines: Vec<ChartRefLineDto>,
    pub totals: Option<Vec<f64>>,
    pub unit: ChartUnitDto,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    pub name: String,
    pub is_default: bool,
}

/// Карточка графика на сетке из 12 колонок; высота ячейки 80 px.
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChartCardDto {
    #[specta(type = specta_typescript::Number)]
    pub id: i64,
    #[specta(type = specta_typescript::Number)]
    pub dashboard_id: i64,
    pub spec: ChartSpecDto,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardPlacementDto {
    pub id: Int53,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

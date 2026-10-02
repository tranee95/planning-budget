//! `ChartSpec` v1 и его валидация.

use serde::{Deserialize, Serialize};

use crate::period::YearMonth;
use crate::query;

/// Фильтр графика не длиннее запроса поиска.
const MAX_FILTER_CHARS: usize = 500;
const MAX_TITLE_CHARS: usize = 100;
const MAX_TOP_N: u32 = 50;
/// Не больше 20 лет месяцев на одном графике.
pub(crate) const MAX_MONTHS: usize = 240;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    Bar,
    StackedBar,
    Hbar,
    Line,
    Area,
    Donut,
    Table,
    Kpi,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
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

/// Единица значений серии.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Rub,
    Percent,
    Count,
}

impl Metric {
    pub fn unit(self) -> Unit {
        match self {
            Self::SavingsRate | Self::LimitUsage => Unit::Percent,
            Self::Count => Unit::Count,
            _ => Unit::Rub,
        }
    }

    /// Считается по тратам, поэтому делится по статусу, типу, категории и тегу.
    pub(crate) fn counts_transactions(self) -> bool {
        matches!(
            self,
            Self::Spent | Self::Expenses | Self::Savings | Self::Count | Self::AvgTicket
        )
    }

    /// Суммируется по периодам и сериям: подходит для накопления, долей и итога.
    pub(crate) fn additive(self) -> bool {
        matches!(
            self,
            Self::Spent | Self::Expenses | Self::Savings | Self::Income | Self::Free | Self::Count
        )
    }

    /// Нужен доход, поэтому есть только по месяцам или итогом.
    fn needs_income(self) -> bool {
        matches!(
            self,
            Self::Income | Self::Free | Self::FreeCum | Self::SavingsCum | Self::SavingsRate
        )
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Spent => "Траты",
            Self::Expenses => "Расходы",
            Self::Income => "Доходы",
            Self::Savings => "Сбережения",
            Self::Free => "Свободный остаток",
            Self::FreeCum => "Остаток накопительно",
            Self::SavingsCum => "Сбережения накопительно",
            Self::SavingsRate => "Норма сбережений",
            Self::LimitUsage => "Факт, % от лимита",
            Self::Count => "Количество трат",
            Self::AvgTicket => "Средняя трата",
        }
    }

    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Spent => "metric.spent",
            Self::Expenses => "metric.expenses",
            Self::Income => "metric.income",
            Self::Savings => "metric.savings",
            Self::Free => "metric.free",
            Self::FreeCum => "metric.free_cum",
            Self::SavingsCum => "metric.savings_cum",
            Self::SavingsRate => "metric.savings_rate",
            Self::LimitUsage => "metric.limit_usage",
            Self::Count => "metric.count",
            Self::AvgTicket => "metric.avg_ticket",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    Month,
    Category,
    Kind,
    Status,
    Tag,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesBy {
    Status,
    Kind,
    Category,
    Metric,
}

impl SeriesBy {
    fn same_axis(self, group: GroupBy) -> bool {
        matches!(
            (self, group),
            (Self::Status, GroupBy::Status)
                | (Self::Kind, GroupBy::Kind)
                | (Self::Category, GroupBy::Category)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodPreset {
    Ytd,
    Last12,
    CurrentMonth,
    All,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeriodSpec {
    Preset { preset: PeriodPreset },
    Range { from: YearMonth, to: YearMonth },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Desc,
    Asc,
    #[default]
    Natural,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub show_limit: bool,
    pub top_n: Option<u32>,
    pub sort: SortOrder,
    pub cumulative: bool,
    pub compare_prev_period: bool,
    pub percent: bool,
}

/// Описание графика; хранится JSON-ом в `charts.spec`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ChartSpec {
    pub version: u8,
    pub title: String,
    #[serde(rename = "type")]
    pub chart_type: ChartType,
    pub metric: Metric,
    pub group_by: GroupBy,
    #[serde(default)]
    pub series_by: Option<SeriesBy>,
    /// Только при `series_by = metric`: каждый показатель — своя серия.
    #[serde(default)]
    pub metrics: Vec<Metric>,
    pub period: PeriodSpec,
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub options: Options,
}

/// Почему спецификацию нельзя построить; тексты показываются в конструкторе подсказкой.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpecError {
    #[error("Неизвестная версия описания графика")]
    Version,
    #[error("Название пустое или длиннее 100 символов")]
    Title,
    #[error("Период задан наоборот или длиннее 20 лет")]
    Period,
    #[error("Строка фильтра длиннее 500 символов")]
    FilterTooLong,
    #[error("Фильтр по источнику в графиках не поддерживается")]
    FilterSource,
    #[error(
        "В фильтре есть нераспознанные условия или свободный текст: графики учитывают только условия вида ключ:значение"
    )]
    FilterInvalid,
    #[error("Кольцевая диаграмма требует разбивку не по месяцам, без серий и без накопления")]
    Donut,
    #[error("Карточка-число считает один показатель без разбивки")]
    Kpi,
    #[error("Столбцы с накоплением нужны только для разбиения на серии")]
    StackedWithoutSeries,
    #[error("Показатель с доходом считается только по месяцам или итогом")]
    NeedsMonthOrTotal,
    #[error("Норма сбережений считается только по месяцам")]
    SavingsRateByMonth,
    #[error("Использование лимита считается только по категориям")]
    LimitUsageByCategory,
    #[error("Использование лимита считается без серий")]
    LimitUsageSeries,
    #[error("Линия лимита есть только у использования лимита")]
    ShowLimit,
    #[error("Серии совпадают с разбивкой")]
    SeriesEqualsGroup,
    #[error("Серии по статусу, типу и категории считаются только по тратам")]
    SeriesNeedTransactions,
    #[error("Для серий по показателям выберите от 2 до 4 показателей одной единицы")]
    MetricsList,
    #[error("Список показателей задаётся только при сериях по показателям")]
    MetricsWithoutSeries,
    #[error("Накопление недоступно для этого показателя или разбивки")]
    Cumulative,
    #[error("Сравнение с прошлым периодом есть только при разбивке по месяцам")]
    ComparePrev,
    #[error("Доли недоступны для этого показателя")]
    Percent,
    #[error("Топ-N — число от 1 до 50")]
    TopN,
    #[error("Топ-N по сериям недоступен для средних значений: хвост нельзя сложить")]
    TopNSeries,
}

impl ChartSpec {
    /// Нужны ли расчёту теги трат: только если фильтр отбирает или исключает по тегам.
    /// Остальным графикам читать теги из базы незачем. `year` — текущий год, как у движка.
    pub fn uses_tags(&self, year: u16) -> bool {
        let filter = query::parse(&self.filter, year).filter;
        !filter.tags.is_empty() || !filter.exclude_tags.is_empty()
    }

    /// Проверяет сочетание полей; данные не читает.
    pub fn validate(&self) -> Result<(), SpecError> {
        if self.version != 1 {
            return Err(SpecError::Version);
        }
        let title_len = self.title.trim().chars().count();
        if title_len == 0 || title_len > MAX_TITLE_CHARS {
            return Err(SpecError::Title);
        }
        if let PeriodSpec::Range { from, to } = self.period
            && (from > to || from.iter_to(to).count() > MAX_MONTHS)
        {
            return Err(SpecError::Period);
        }
        self.validate_filter()?;
        if self
            .options
            .top_n
            .is_some_and(|n| !(1..=MAX_TOP_N).contains(&n))
        {
            return Err(SpecError::TopN);
        }

        let metrics = self.effective_metrics()?;
        for metric in &metrics {
            self.validate_metric(*metric)?;
        }
        self.validate_type()?;
        self.validate_options(&metrics)
    }

    /// Показатели графика: один или список при `series_by = metric`.
    pub(crate) fn effective_metrics(&self) -> Result<Vec<Metric>, SpecError> {
        if self.series_by == Some(SeriesBy::Metric) {
            let first_unit = self.metrics.first().map(|m| m.unit());
            let same_unit = self.metrics.iter().all(|m| Some(m.unit()) == first_unit);
            if !(2..=4).contains(&self.metrics.len()) || !same_unit {
                return Err(SpecError::MetricsList);
            }
            Ok(self.metrics.clone())
        } else if self.metrics.is_empty() {
            Ok(vec![self.metric])
        } else {
            Err(SpecError::MetricsWithoutSeries)
        }
    }

    fn validate_filter(&self) -> Result<(), SpecError> {
        if self.filter.chars().count() > MAX_FILTER_CHARS {
            return Err(SpecError::FilterTooLong);
        }
        // Год нужен только токенам «мес:сен»; на проверку источника он не влияет.
        let parsed = query::parse(&self.filter, 2026);
        if parsed.filter.source.is_some() {
            return Err(SpecError::FilterSource);
        }
        // Текст и нераспознанные токены поиск бы учёл, а график молча отбросил бы.
        if !parsed.errors.is_empty() || !parsed.text.is_empty() {
            return Err(SpecError::FilterInvalid);
        }
        Ok(())
    }

    fn validate_metric(&self, metric: Metric) -> Result<(), SpecError> {
        let group = self.group_by;
        if metric.needs_income() && !matches!(group, GroupBy::Month | GroupBy::None) {
            return Err(SpecError::NeedsMonthOrTotal);
        }
        if metric == Metric::SavingsRate && group != GroupBy::Month {
            return Err(SpecError::SavingsRateByMonth);
        }
        if metric == Metric::LimitUsage {
            if group != GroupBy::Category {
                return Err(SpecError::LimitUsageByCategory);
            }
            if self.series_by.is_some() {
                return Err(SpecError::LimitUsageSeries);
            }
        }
        if let Some(series) = self.series_by {
            if series.same_axis(group) {
                return Err(SpecError::SeriesEqualsGroup);
            }
            if series != SeriesBy::Metric && !metric.counts_transactions() {
                return Err(SpecError::SeriesNeedTransactions);
            }
        }
        Ok(())
    }

    fn validate_type(&self) -> Result<(), SpecError> {
        match self.chart_type {
            ChartType::Donut
                if matches!(self.group_by, GroupBy::Month | GroupBy::None)
                    || self.series_by.is_some()
                    || self.options.cumulative =>
            {
                Err(SpecError::Donut)
            }
            ChartType::Kpi if self.group_by != GroupBy::None || self.series_by.is_some() => {
                Err(SpecError::Kpi)
            }
            ChartType::StackedBar
                if !matches!(
                    self.series_by,
                    Some(SeriesBy::Status | SeriesBy::Kind | SeriesBy::Category)
                ) =>
            {
                Err(SpecError::StackedWithoutSeries)
            }
            _ => Ok(()),
        }
    }

    fn validate_options(&self, metrics: &[Metric]) -> Result<(), SpecError> {
        let options = &self.options;
        if options.top_n.is_some()
            && self.series_by == Some(SeriesBy::Category)
            && metrics.iter().any(|m| !m.additive())
        {
            return Err(SpecError::TopNSeries);
        }
        if options.show_limit && !metrics.contains(&Metric::LimitUsage) {
            return Err(SpecError::ShowLimit);
        }
        if options.cumulative
            && (self.group_by != GroupBy::Month || metrics.iter().any(|m| !m.additive()))
        {
            return Err(SpecError::Cumulative);
        }
        if options.compare_prev_period && self.group_by != GroupBy::Month {
            return Err(SpecError::ComparePrev);
        }
        if options.percent
            && (self.series_by == Some(SeriesBy::Metric)
                || metrics
                    .iter()
                    .any(|m| !m.additive() || m.unit() == Unit::Percent))
        {
            return Err(SpecError::Percent);
        }
        Ok(())
    }
}

impl SpecError {
    /// Ключ причины для интерфейса (тексты лежат на фронтенде).
    pub fn key(self) -> &'static str {
        match self {
            Self::Version => "chart.version",
            Self::Title => "chart.title",
            Self::Period => "chart.period",
            Self::FilterTooLong => "chart.filter_too_long",
            Self::FilterSource => "chart.filter_source",
            Self::FilterInvalid => "chart.filter_invalid",
            Self::Donut => "chart.donut",
            Self::Kpi => "chart.kpi",
            Self::StackedWithoutSeries => "chart.stacked_without_series",
            Self::NeedsMonthOrTotal => "chart.needs_month_or_total",
            Self::SavingsRateByMonth => "chart.savings_rate_by_month",
            Self::LimitUsageByCategory => "chart.limit_usage_by_category",
            Self::LimitUsageSeries => "chart.limit_usage_series",
            Self::ShowLimit => "chart.show_limit",
            Self::SeriesEqualsGroup => "chart.series_equals_group",
            Self::SeriesNeedTransactions => "chart.series_need_transactions",
            Self::MetricsList => "chart.metrics_list",
            Self::MetricsWithoutSeries => "chart.metrics_without_series",
            Self::Cumulative => "chart.cumulative",
            Self::ComparePrev => "chart.compare_prev",
            Self::Percent => "chart.percent",
            Self::TopN => "chart.top_n",
            Self::TopNSeries => "chart.top_n_series",
        }
    }
}

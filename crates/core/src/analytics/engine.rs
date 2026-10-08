//! Движок графиков: фильтр → ячейки `(подпись, серия)` → `ChartData`.

use std::collections::{BTreeMap, BTreeSet};

use super::matcher::Matcher;
use super::spec::MAX_MONTHS;
use super::{
    AnalyticsError, ChartData, ChartSpec, Context, GroupBy, Metric, PeriodPreset, PeriodSpec,
    RefLine, Series, SeriesBy, SortOrder, Unit,
};
use crate::calc::{Ledger, usage};
use crate::error::{CoreError, MoneyError};
use crate::model::{Category, CategoryId, CategoryKind, DataSet, Income, Transaction, TxStatus};
use crate::money::Money;
use crate::period::YearMonth;
use crate::query;

/// Ось графика: подпись по горизонтали или серия.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Dim {
    Total,
    Month(YearMonth),
    Category(CategoryId),
    Kind(CategoryKind),
    Status(TxStatus),
    Tag(String),
    Metric(Metric),
    /// Хвост за пределами «топ-N».
    Other,
}

type Cells = BTreeMap<(Dim, Dim), Option<f64>>;

struct FilteredTx<'a> {
    tx: &'a Transaction,
    category: &'a Category,
    tags: &'a [String],
}

#[derive(Default)]
struct Cell {
    sum: Money,
    count: u32,
}

impl Cell {
    fn add(&mut self, amount: Money) -> Result<(), MoneyError> {
        self.sum = self.sum.checked_add(amount)?;
        self.count = self.count.saturating_add(1);
        Ok(())
    }

    fn value(&self, metric: Metric) -> Option<f64> {
        match metric {
            Metric::Count => Some(f64::from(self.count)),
            Metric::AvgTicket if self.count == 0 => None,
            Metric::AvgTicket => Some(rub(self.sum) / f64::from(self.count)),
            _ => Some(rub(self.sum)),
        }
    }
}

/// Доходы и траты месяца; траты уже разделены на расходы и сбережения.
#[derive(Default, Clone, Copy)]
struct Row {
    income: Money,
    expenses: Money,
    savings: Money,
}

impl Row {
    fn free(&self) -> Result<Money, MoneyError> {
        self.income
            .checked_sub(self.expenses)?
            .checked_sub(self.savings)
    }

    fn add(&mut self, other: &Self) -> Result<(), MoneyError> {
        self.income = self.income.checked_add(other.income)?;
        self.expenses = self.expenses.checked_add(other.expenses)?;
        self.savings = self.savings.checked_add(other.savings)?;
        Ok(())
    }
}

struct Assembled {
    labels: Vec<String>,
    tokens: Vec<String>,
    series: Vec<(Dim, Series)>,
}

pub(super) struct Engine<'a> {
    data: &'a DataSet,
    spec: &'a ChartSpec,
    ledger: &'a Ledger<'a>,
    matcher: Matcher,
    txs: Vec<FilteredTx<'a>>,
    incomes: Vec<&'a Income>,
    months: Vec<YearMonth>,
    metrics: Vec<Metric>,
    category_order: BTreeMap<CategoryId, i64>,
}

fn rub(money: Money) -> f64 {
    money.as_f64() / 100.0
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn in_range(month: YearMonth, months: &[YearMonth]) -> bool {
    match (months.first(), months.last()) {
        (Some(lo), Some(hi)) => *lo <= month && month <= *hi,
        _ => false,
    }
}

/// Границы периода, известные без данных; `None` у пресета `all`.
pub(super) fn period_bounds(
    period: PeriodSpec,
    today: YearMonth,
) -> Option<(YearMonth, YearMonth)> {
    match period {
        PeriodSpec::Range { from, to } => Some((from, to)),
        PeriodSpec::Preset { preset } => match preset {
            PeriodPreset::Ytd => Some((
                YearMonth::first_of_year(today.year()).unwrap_or(today),
                today,
            )),
            PeriodPreset::Last12 => Some((
                (0..11).try_fold(today, |m, _| m.pred()).unwrap_or(today),
                today,
            )),
            PeriodPreset::CurrentMonth => Some((today, today)),
            PeriodPreset::All => None,
        },
    }
}

pub(super) fn period_months(
    data: &DataSet,
    period: PeriodSpec,
    today: YearMonth,
) -> Vec<YearMonth> {
    let (from, to) = period_bounds(period, today).unwrap_or_else(|| {
        let all = || {
            data.transactions
                .iter()
                .map(|t| t.month)
                .chain(data.incomes.iter().map(|i| i.month))
        };
        (all().min().unwrap_or(today), all().max().unwrap_or(today))
    });
    // При слишком длинной истории остаются самые свежие месяцы.
    let all_months: Vec<YearMonth> = from.iter_to(to).collect();
    let skip = all_months.len().saturating_sub(MAX_MONTHS);
    let months: Vec<YearMonth> = all_months.into_iter().skip(skip).collect();
    if months.is_empty() {
        vec![today]
    } else {
        months
    }
}

fn kind_label(kind: CategoryKind) -> (&'static str, &'static str) {
    match kind {
        CategoryKind::Mandatory => ("Обязательные", "kind.mandatory"),
        CategoryKind::Wants => ("Желания", "kind.wants"),
        CategoryKind::Savings => ("Сбережения", "kind.savings"),
        CategoryKind::Loans => ("Займы", "kind.loans"),
    }
}

fn status_label(status: TxStatus) -> (&'static str, &'static str) {
    match status {
        TxStatus::Paid => ("Оплачено", "status.paid"),
        TxStatus::Debt => ("Долг", "status.debt"),
        TxStatus::Unplanned => ("Внеплановые", "status.unplanned"),
        TxStatus::Planned => ("План", "status.planned"),
    }
}

fn kind_index(kind: CategoryKind) -> i64 {
    match kind {
        CategoryKind::Mandatory => 0,
        CategoryKind::Wants => 1,
        CategoryKind::Savings => 2,
        CategoryKind::Loans => 3,
    }
}

fn status_index(status: TxStatus) -> i64 {
    match status {
        TxStatus::Paid => 0,
        TxStatus::Debt => 1,
        TxStatus::Unplanned => 2,
        TxStatus::Planned => 3,
    }
}

impl<'a> Engine<'a> {
    pub fn new(
        data: &'a DataSet,
        spec: &'a ChartSpec,
        ctx: &Context<'a>,
        ledger: &'a Ledger<'a>,
    ) -> Result<Self, AnalyticsError> {
        let months = period_months(data, spec.period, ctx.today);
        // Набор данных может быть шире периода (он общий для нескольких графиков): записи вне
        // периода и предыдущего периода для сравнения не фильтруем и не считаем.
        let first = months.first().copied().unwrap_or(ctx.today);
        let last = months.last().copied().unwrap_or(ctx.today);
        let from = if spec.options.compare_prev_period {
            (0..months.len())
                .try_fold(first, |m, _| m.pred())
                .unwrap_or(first)
        } else {
            first
        };
        let matcher = Matcher::new(query::parse(&spec.filter, ctx.today.year()).filter, data);
        let categories: BTreeMap<CategoryId, &Category> =
            data.categories.iter().map(|c| (c.id, c)).collect();
        let mut txs = Vec::new();
        for tx in data
            .transactions
            .iter()
            .filter(|tx| from <= tx.month && tx.month <= last)
        {
            let category = categories
                .get(&tx.category_id)
                .ok_or(CoreError::CategoryNotFound(tx.category_id.0))?;
            let tags = ctx.tx_tags.get(&tx.id).map_or(&[][..], Vec::as_slice);
            if matcher.transaction(tx, category, tags) {
                txs.push(FilteredTx { tx, category, tags });
            }
        }
        let incomes = data
            .incomes
            .iter()
            .filter(|i| from <= i.month && i.month <= last && matcher.income(i))
            .collect();

        let mut sorted: Vec<&Category> = data.categories.iter().collect();
        sorted.sort_by_key(|c| (c.sort_order, c.id));
        let category_order = (0_i64..).zip(sorted).map(|(i, c)| (c.id, i)).collect();

        Ok(Self {
            data,
            spec,
            ledger,
            matcher,
            txs,
            incomes,
            months,
            metrics: spec.effective_metrics()?,
            category_order,
        })
    }

    pub fn run(&self) -> Result<ChartData, AnalyticsError> {
        let options = &self.spec.options;
        let main = self.assemble(&self.months)?;
        let mut series: Vec<Series> = main.series.iter().map(|(_, s)| s.clone()).collect();

        let prev_months = if options.compare_prev_period {
            self.previous_months()
        } else {
            None
        };
        if let Some(prev_months) = prev_months {
            let prev = self.assemble(&prev_months)?;
            for (dim, current) in &main.series {
                let values = prev.series.iter().find(|(d, _)| d == dim).map_or_else(
                    || vec![None; current.values.len()],
                    |(_, s)| s.values.clone(),
                );
                series.push(Series {
                    name: format!("{} (прошлый период)", current.name),
                    color: format!("{}.prev", current.color),
                    values,
                });
            }
        }

        let first = self.metrics.first().copied().unwrap_or(self.spec.metric);
        let unit = if options.percent {
            Unit::Percent
        } else {
            first.unit()
        };
        let summable =
            !options.percent && self.spec.series_by != Some(SeriesBy::Metric) && first.additive();
        let totals = summable.then(|| {
            let mut totals = vec![0.0; main.labels.len()];
            for (_, s) in &main.series {
                for (t, v) in totals.iter_mut().zip(&s.values) {
                    *t += v.unwrap_or(0.0);
                }
            }
            totals.into_iter().map(round2).collect()
        });

        Ok(ChartData {
            categories: main.labels,
            category_tokens: main.tokens,
            series,
            reference_lines: self.reference_lines(),
            totals,
            unit,
        })
    }

    fn previous_months(&self) -> Option<Vec<YearMonth>> {
        let n = self.months.len();
        let first = self.months.first()?;
        let start = (0..n).try_fold(*first, |m, _| m.pred())?;
        Some(
            std::iter::successors(Some(start), |m| m.succ())
                .take(n)
                .collect(),
        )
    }

    fn reference_lines(&self) -> Vec<RefLine> {
        let settings = &self.data.settings;
        let mut lines = Vec::new();
        if self.metrics.contains(&Metric::SavingsRate) {
            lines.push(RefLine {
                name: "Коридор сбережений".to_owned(),
                from: settings.savings_min.as_ratio() * 100.0,
                to: Some(settings.savings_max.as_ratio() * 100.0),
            });
        }
        if self.spec.options.show_limit {
            lines.push(RefLine {
                name: "Лимит".to_owned(),
                from: 100.0,
                to: None,
            });
        }
        lines
    }

    // ---- ячейки ----

    fn cells(&self, months: &[YearMonth]) -> Result<Cells, AnalyticsError> {
        let mut cells = Cells::new();
        if self.spec.series_by == Some(SeriesBy::Metric) {
            for metric in &self.metrics {
                self.metric_cells(*metric, months, Some(Dim::Metric(*metric)), &mut cells)?;
            }
        } else {
            self.metric_cells(self.spec.metric, months, None, &mut cells)?;
        }
        Ok(cells)
    }

    /// `series` задан, когда серия — сам показатель; иначе серию определяет `series_by`.
    fn metric_cells(
        &self,
        metric: Metric,
        months: &[YearMonth],
        series: Option<Dim>,
        cells: &mut Cells,
    ) -> Result<(), AnalyticsError> {
        match metric {
            Metric::LimitUsage => self.limit_usage_cells(months, cells),
            m if m.counts_transactions() => self.transaction_cells(m, months, series, cells),
            m => self.month_cells(m, months, series.unwrap_or(Dim::Total), cells),
        }
    }

    fn series_dim(&self, f: &FilteredTx<'_>) -> Dim {
        match self.spec.series_by {
            Some(SeriesBy::Status) => Dim::Status(f.tx.status),
            Some(SeriesBy::Kind) => Dim::Kind(f.category.kind),
            Some(SeriesBy::Category) => Dim::Category(f.category.id),
            Some(SeriesBy::Metric) | None => Dim::Total,
        }
    }

    fn group_dims(&self, f: &FilteredTx<'_>) -> Vec<Dim> {
        match self.spec.group_by {
            GroupBy::Month => vec![Dim::Month(f.tx.month)],
            GroupBy::Category => vec![Dim::Category(f.category.id)],
            GroupBy::Kind => vec![Dim::Kind(f.category.kind)],
            GroupBy::Status => vec![Dim::Status(f.tx.status)],
            GroupBy::Tag if f.tags.is_empty() => vec![Dim::Tag(String::new())],
            GroupBy::Tag => f.tags.iter().cloned().map(Dim::Tag).collect(),
            GroupBy::None => vec![Dim::Total],
        }
    }

    fn transaction_cells(
        &self,
        metric: Metric,
        months: &[YearMonth],
        series: Option<Dim>,
        cells: &mut Cells,
    ) -> Result<(), AnalyticsError> {
        let mut acc: BTreeMap<(Dim, Dim), Cell> = BTreeMap::new();
        for f in self.txs.iter().filter(|f| in_range(f.tx.month, months)) {
            let is_savings = f.category.kind == CategoryKind::Savings;
            if (metric == Metric::Expenses && is_savings)
                || (metric == Metric::Savings && !is_savings)
            {
                continue;
            }
            let series = series.clone().unwrap_or_else(|| self.series_dim(f));
            for group in self.group_dims(f) {
                acc.entry((group, series.clone()))
                    .or_default()
                    .add(f.tx.amount)?;
            }
        }
        cells.extend(acc.into_iter().map(|(key, cell)| (key, cell.value(metric))));
        Ok(())
    }

    fn month_cells(
        &self,
        metric: Metric,
        months: &[YearMonth],
        series: Dim,
        cells: &mut Cells,
    ) -> Result<(), AnalyticsError> {
        let mut rows: BTreeMap<YearMonth, Row> =
            months.iter().map(|m| (*m, Row::default())).collect();
        for income in self.incomes.iter().filter(|i| in_range(i.month, months)) {
            if let Some(row) = rows.get_mut(&income.month) {
                row.income = row.income.checked_add(income.amount)?;
            }
        }
        for f in self.txs.iter().filter(|f| in_range(f.tx.month, months)) {
            if let Some(row) = rows.get_mut(&f.tx.month) {
                let slot = if f.category.kind == CategoryKind::Savings {
                    &mut row.savings
                } else {
                    &mut row.expenses
                };
                *slot = slot.checked_add(f.tx.amount)?;
            }
        }

        let mut running = Row::default();
        let mut free_cum = Money::ZERO;
        for (month, row) in &rows {
            running.add(row)?;
            free_cum = free_cum.checked_add(row.free()?)?;
            if self.spec.group_by == GroupBy::Month {
                cells.insert(
                    (Dim::Month(*month), series.clone()),
                    month_value(metric, row, free_cum, running.savings)?,
                );
            }
        }
        if self.spec.group_by != GroupBy::Month {
            cells.insert(
                (Dim::Total, series),
                month_value(metric, &running, free_cum, running.savings)?,
            );
        }
        Ok(())
    }

    fn limit_usage_cells(
        &self,
        months: &[YearMonth],
        cells: &mut Cells,
    ) -> Result<(), AnalyticsError> {
        let mut facts: BTreeMap<CategoryId, Money> = BTreeMap::new();
        for f in self.txs.iter().filter(|f| in_range(f.tx.month, months)) {
            let slot = facts.entry(f.category.id).or_default();
            *slot = slot.checked_add(f.tx.amount)?;
        }
        for category in &self.data.categories {
            if category.kind == CategoryKind::Savings || !self.matcher.category(category) {
                continue;
            }
            let fact = facts.get(&category.id).copied().unwrap_or_default();
            if category.archived && fact.is_zero() {
                continue;
            }
            let mut limit: Option<Money> = None;
            for month in months {
                if let Some(l) = self.ledger.limit(*month, category.id)? {
                    limit = Some(limit.unwrap_or_default().checked_add(l)?);
                }
            }
            if let Some(limit) = limit {
                cells.insert(
                    (Dim::Category(category.id), Dim::Total),
                    Some(usage(fact, limit) * 100.0),
                );
            }
        }
        Ok(())
    }

    // ---- сборка ----

    fn rank(&self, dim: &Dim) -> (i64, String) {
        match dim {
            Dim::Month(m) => (
                i64::from(m.year()) * 12 + i64::from(m.month()),
                String::new(),
            ),
            Dim::Category(id) => (
                self.category_order.get(id).copied().unwrap_or(i64::MAX - 1),
                String::new(),
            ),
            Dim::Kind(k) => (kind_index(*k), String::new()),
            Dim::Status(s) => (status_index(*s), String::new()),
            Dim::Tag(t) => (0, t.to_lowercase()),
            Dim::Total | Dim::Metric(_) => (0, String::new()),
            Dim::Other => (i64::MAX, String::new()),
        }
    }

    /// Подпись и токен цвета оси.
    fn describe(&self, dim: &Dim, with_year: bool) -> (String, String) {
        match dim {
            Dim::Total => (
                self.spec.metric.label().to_owned(),
                self.spec.metric.token().to_owned(),
            ),
            Dim::Month(m) => (
                if with_year {
                    m.ru_name()
                } else {
                    m.month_name_ru().to_owned()
                },
                format!("month:{m}"),
            ),
            Dim::Category(id) => {
                let name = self
                    .data
                    .categories
                    .iter()
                    .find(|c| c.id == *id)
                    .map_or_else(String::new, |c| c.name.clone());
                (name, format!("category:{}", id.0))
            }
            Dim::Kind(k) => {
                let (name, token) = kind_label(*k);
                (name.to_owned(), token.to_owned())
            }
            Dim::Status(s) => {
                let (name, token) = status_label(*s);
                (name.to_owned(), token.to_owned())
            }
            Dim::Tag(t) if t.is_empty() => ("Без тега".to_owned(), String::new()),
            Dim::Tag(t) => (format!("#{t}"), String::new()),
            Dim::Metric(m) => (m.label().to_owned(), m.token().to_owned()),
            Dim::Other => ("Прочее".to_owned(), "other".to_owned()),
        }
    }

    /// Нет данных: суммы равны нулю, а среднее и доля не определены.
    fn missing(&self, series: &Dim) -> Option<f64> {
        let metric = match series {
            Dim::Metric(m) => *m,
            _ => self.spec.metric,
        };
        (!matches!(metric, Metric::AvgTicket | Metric::SavingsRate)).then_some(0.0)
    }

    /// Серии вне «топ-N» по категориям сливаются в «Прочее».
    fn merge_tail(&self, cells: Cells, top: usize) -> Cells {
        let mut totals: BTreeMap<Dim, f64> = BTreeMap::new();
        for ((_, series), value) in &cells {
            *totals.entry(series.clone()).or_insert(0.0) += value.unwrap_or(0.0);
        }
        let mut ranked: Vec<(Dim, f64)> = totals.into_iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        let keep: BTreeSet<Dim> = ranked.into_iter().take(top).map(|(d, _)| d).collect();

        let mut merged = Cells::new();
        for ((group, series), value) in cells {
            let series = if keep.contains(&series) {
                series
            } else {
                Dim::Other
            };
            let slot = merged.entry((group, series)).or_insert(None);
            *slot = match (*slot, value) {
                (Some(a), Some(b)) => Some(a + b),
                (a, b) => a.or(b),
            };
        }
        merged
    }

    fn assemble(&self, months: &[YearMonth]) -> Result<Assembled, AnalyticsError> {
        let options = &self.spec.options;
        let mut cells = self.cells(months)?;
        if let (Some(SeriesBy::Category), Some(top)) = (self.spec.series_by, options.top_n) {
            cells = self.merge_tail(cells, usize::try_from(top).unwrap_or(usize::MAX));
        }

        let series_dims: Vec<Dim> = match self.spec.series_by {
            Some(SeriesBy::Metric) => self.metrics.iter().map(|m| Dim::Metric(*m)).collect(),
            None => vec![Dim::Total],
            Some(_) => {
                let mut dims: Vec<Dim> = cells
                    .keys()
                    .map(|(_, s)| s.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                dims.sort_by_key(|d| self.rank(d));
                dims
            }
        };

        let groups = self.group_axis(&cells, &series_dims, months);
        let with_year = match (months.first(), months.last()) {
            (Some(a), Some(b)) => a.year() != b.year(),
            _ => false,
        };

        let mut series: Vec<(Dim, Vec<Option<f64>>)> = series_dims
            .into_iter()
            .map(|dim| {
                let values = groups
                    .iter()
                    .map(|g| {
                        cells
                            .get(&(g.clone(), dim.clone()))
                            .copied()
                            .unwrap_or_else(|| self.missing(&dim))
                    })
                    .collect();
                (dim, values)
            })
            .collect();

        if options.cumulative {
            for (_, values) in &mut series {
                let mut running = 0.0;
                for value in values.iter_mut().flatten() {
                    running += *value;
                    *value = running;
                }
            }
        }
        if options.percent {
            self.to_percent(&mut series, groups.len());
        }

        let round = !options.percent && self.metrics.first().is_some_and(|m| m.unit() == Unit::Rub);
        let described = series
            .into_iter()
            .map(|(dim, values)| {
                let (name, color) = self.describe(&dim, with_year);
                let values = if round {
                    values.into_iter().map(|v| v.map(round2)).collect()
                } else {
                    values
                };
                (
                    dim,
                    Series {
                        name,
                        color,
                        values,
                    },
                )
            })
            .collect();
        let (labels, tokens) = groups
            .iter()
            .map(|g| {
                let (name, token) = self.describe(g, with_year);
                (name, token)
            })
            .unzip();
        Ok(Assembled {
            labels,
            tokens,
            series: described,
        })
    }

    /// Доли: при сериях — каждая колонка в сумме 100%, иначе — доля серии в её итоге.
    fn to_percent(&self, series: &mut [(Dim, Vec<Option<f64>>)], columns: usize) {
        let share = |value: &mut Option<f64>, total: f64| {
            *value = value
                .filter(|_| total.abs() > f64::EPSILON)
                .map(|v| v / total * 100.0);
        };
        if self.spec.series_by.is_some() {
            let mut totals = vec![0.0; columns];
            for (_, values) in series.iter() {
                for (t, v) in totals.iter_mut().zip(values) {
                    *t += v.unwrap_or(0.0);
                }
            }
            for (_, values) in series.iter_mut() {
                for (v, t) in values.iter_mut().zip(&totals) {
                    share(v, *t);
                }
            }
        } else {
            for (_, values) in series.iter_mut() {
                let total: f64 = values.iter().flatten().sum();
                for v in values.iter_mut() {
                    share(v, total);
                }
            }
        }
    }

    /// Подписи оси: месяцы периода, единственный итог или категории/типы/статусы/теги
    /// с данными — в порядке сортировки, обрезанные по «топ-N».
    fn group_axis(&self, cells: &Cells, series: &[Dim], months: &[YearMonth]) -> Vec<Dim> {
        match self.spec.group_by {
            GroupBy::Month => return months.iter().map(|m| Dim::Month(*m)).collect(),
            GroupBy::None => return vec![Dim::Total],
            _ => {}
        }
        let options = &self.spec.options;
        let present: BTreeSet<&Dim> = cells.keys().map(|(g, _)| g).collect();
        let total = |group: &Dim| -> f64 {
            series
                .iter()
                .filter_map(|s| cells.get(&((*group).clone(), s.clone())).copied().flatten())
                .sum()
        };
        let mut groups: Vec<(Dim, f64)> =
            present.into_iter().map(|g| (g.clone(), total(g))).collect();

        let by_value = |a: &(Dim, f64), b: &(Dim, f64)| b.1.total_cmp(&a.1);
        if let (Some(top), false) = (
            options.top_n,
            self.spec.series_by == Some(SeriesBy::Category),
        ) {
            groups.sort_by(by_value);
            groups.truncate(usize::try_from(top).unwrap_or(usize::MAX));
        }
        match options.sort {
            SortOrder::Desc => groups.sort_by(by_value),
            SortOrder::Asc => groups.sort_by(|a, b| a.1.total_cmp(&b.1)),
            SortOrder::Natural => groups.sort_by_key(|(g, _)| self.rank(g)),
        }
        groups.into_iter().map(|(g, _)| g).collect()
    }
}

fn month_value(
    metric: Metric,
    row: &Row,
    free_cum: Money,
    savings_cum: Money,
) -> Result<Option<f64>, MoneyError> {
    Ok(match metric {
        Metric::Income => Some(rub(row.income)),
        Metric::Free => Some(rub(row.free()?)),
        Metric::FreeCum => Some(rub(free_cum)),
        Metric::SavingsCum => Some(rub(savings_cum)),
        Metric::SavingsRate => row.savings.ratio(row.income).map(|r| r * 100.0),
        _ => None,
    })
}

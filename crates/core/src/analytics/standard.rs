//! Стандартный дашборд «Мой бюджет»: единственное описание для сида и тестов.

use super::{
    ChartSpec, ChartType, GroupBy, Metric, Options, PeriodPreset, PeriodSpec, SeriesBy, SortOrder,
};

/// Карточка на сетке из 12 колонок; высота ячейки 80 px.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Placement {
    pub spec: ChartSpec,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

const LARGE_CATEGORIES: &str =
    r#"кат:продукты,одежда,"кафе и доставка","хобби и игры","дом и техника""#;

fn spec(
    title: &str,
    chart_type: ChartType,
    metric: Metric,
    group_by: GroupBy,
    preset: PeriodPreset,
) -> ChartSpec {
    ChartSpec {
        version: 1,
        title: title.to_owned(),
        chart_type,
        metric,
        group_by,
        series_by: None,
        metrics: Vec::new(),
        period: PeriodSpec::Preset { preset },
        filter: String::new(),
        options: Options::default(),
    }
}

fn place(spec: ChartSpec, x: u8, y: u8, w: u8, h: u8) -> Placement {
    Placement { spec, x, y, w, h }
}

/// Девять графиков: семь из листа «Графики» старой таблицы, «Лимит и факт» и «Норма сбережений».
pub fn standard_dashboard() -> Vec<Placement> {
    use ChartType::{Bar, Donut, Hbar, Line, StackedBar};
    use GroupBy::{Category, Kind, Month};
    use PeriodPreset::{CurrentMonth, Ytd};

    let income_expenses_savings = ChartSpec {
        series_by: Some(SeriesBy::Metric),
        metrics: vec![Metric::Income, Metric::Expenses, Metric::Savings],
        ..spec(
            "Доходы, расходы и сбережения",
            Bar,
            Metric::Income,
            Month,
            Ytd,
        )
    };
    let balance = ChartSpec {
        series_by: Some(SeriesBy::Metric),
        metrics: vec![Metric::FreeCum, Metric::SavingsCum],
        ..spec(
            "Накопления: остаток и сбережения",
            Line,
            Metric::FreeCum,
            Month,
            Ytd,
        )
    };
    let by_status = ChartSpec {
        series_by: Some(SeriesBy::Status),
        ..spec("Траты по статусам", StackedBar, Metric::Spent, Month, Ytd)
    };
    let by_kind = ChartSpec {
        series_by: Some(SeriesBy::Kind),
        ..spec(
            "Обязательные / желания / сбережения",
            StackedBar,
            Metric::Spent,
            Month,
            Ytd,
        )
    };
    let by_category = ChartSpec {
        options: Options {
            sort: SortOrder::Desc,
            ..Options::default()
        },
        ..spec(
            "Расходы за год по категориям",
            Hbar,
            Metric::Expenses,
            Category,
            Ytd,
        )
    };
    let structure = spec("Структура по типам", Donut, Metric::Spent, Kind, Ytd);
    let large = ChartSpec {
        series_by: Some(SeriesBy::Category),
        filter: LARGE_CATEGORIES.to_owned(),
        ..spec(
            "Динамика крупных категорий",
            Line,
            Metric::Expenses,
            Month,
            Ytd,
        )
    };
    let limit_and_fact = ChartSpec {
        options: Options {
            show_limit: true,
            sort: SortOrder::Desc,
            ..Options::default()
        },
        ..spec(
            "Лимит и факт (текущий месяц)",
            Hbar,
            Metric::LimitUsage,
            Category,
            CurrentMonth,
        )
    };
    let savings_rate = spec("Норма сбережений", Line, Metric::SavingsRate, Month, Ytd);

    vec![
        place(income_expenses_savings, 0, 0, 6, 4),
        place(balance, 6, 0, 6, 4),
        place(by_status, 0, 4, 6, 4),
        place(by_kind, 6, 4, 6, 4),
        place(by_category, 0, 8, 6, 5),
        place(structure, 6, 8, 6, 5),
        place(large, 0, 13, 12, 4),
        place(limit_and_fact, 0, 17, 12, 5),
        place(savings_rate, 0, 22, 6, 3),
    ]
}

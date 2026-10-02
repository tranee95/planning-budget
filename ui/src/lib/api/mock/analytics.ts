import type { ChartDataDto, ChartSpecDto } from '../bindings';

/**
 * Мок `analytics_run`: правдоподобная форма ответа для экранов и скриншотов. Числа не считаются
 * по данным — настоящие формулы и фильтры живут в Rust (`core::analytics`).
 */

const MONTHS = ['Январь', 'Февраль', 'Март', 'Апрель', 'Май', 'Июнь', 'Июль', 'Август', 'Сентябрь'];
const CATEGORIES: [string, string][] = [
  ['Продукты', 'category:1'],
  ['Маркетплейсы', 'category:11'],
  ['Одежда', 'category:10'],
  ['Хобби и игры', 'category:12'],
  ['Дом и техника', 'category:13']
];
const KINDS: [string, string][] = [
  ['Обязательные', 'kind.mandatory'],
  ['Желания', 'kind.wants'],
  ['Сбережения', 'kind.savings']
];
const STATUSES: [string, string][] = [
  ['Оплачено', 'status.paid'],
  ['Долг', 'status.debt'],
  ['Внеплановые', 'status.unplanned'],
  ['План', 'status.planned']
];
const METRIC_LABELS: Record<string, string> = {
  spent: 'Траты',
  expenses: 'Расходы',
  income: 'Доходы',
  savings: 'Сбережения',
  free: 'Свободный остаток',
  free_cum: 'Остаток накопительно',
  savings_cum: 'Сбережения накопительно',
  savings_rate: 'Норма сбережений',
  limit_usage: 'Факт, % от лимита',
  count: 'Количество трат',
  avg_ticket: 'Средняя трата'
};

/** Детерминированное «шумное» значение: одни и те же числа между прогонами и снимками. */
const wave = (a: number, b: number, base: number): number =>
  Math.round(base * (0.6 + 0.4 * Math.abs(Math.sin(a * 1.7 + b * 2.3 + 1))) * 100) / 100;

export function mockAnalyticsRun(spec: ChartSpecDto): ChartDataDto {
  const percentMetric = spec.metric === 'savings_rate' || spec.metric === 'limit_usage';
  const axis: [string, string][] =
    spec.groupBy === 'month'
      ? MONTHS.map((m) => [m, ''])
      : spec.groupBy === 'category'
        ? CATEGORIES
        : spec.groupBy === 'kind'
          ? KINDS
          : spec.groupBy === 'status'
            ? STATUSES
            : [['Итого', '']];
  const series: [string, string][] =
    spec.seriesBy === 'metric'
      ? spec.metrics.map((m) => [METRIC_LABELS[m] ?? m, `metric.${m}`])
      : spec.seriesBy === 'status'
        ? STATUSES
        : spec.seriesBy === 'kind'
          ? KINDS
          : spec.seriesBy === 'category'
            ? CATEGORIES
            : [[METRIC_LABELS[spec.metric] ?? spec.metric, `metric.${spec.metric}`]];
  const base = percentMetric ? 14 : spec.metric === 'count' ? 40 : 60_000;
  const values = series.map((_, s) => axis.map((_, i) => wave(s, i, base)));
  const totals = values[0]?.map((_, i) => values.reduce((sum, row) => sum + (row[i] ?? 0), 0));
  return {
    categories: axis.map(([name]) => name),
    categoryTokens: axis.map(([, token]) => token),
    series: series.map(([name, color], s) => ({ name, color, values: values[s] ?? [] })),
    referenceLines: percentMetric
      ? [
          {
            name: spec.options.showLimit ? 'Лимит' : 'Коридор сбережений',
            from: 13,
            to: spec.options.showLimit ? null : 15
          }
        ]
      : [],
    totals: percentMetric ? null : (totals ?? null),
    unit: percentMetric ? 'percent' : spec.metric === 'count' ? 'count' : 'rub'
  };
}

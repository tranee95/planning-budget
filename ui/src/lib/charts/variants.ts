import type {
  ChartGroupByDto,
  ChartMetricDto,
  ChartSeriesByDto,
  ChartSpecDto,
  ChartTypeDto
} from '$lib/api/bindings';

/** Значения полей конструктора и их подписи. */

export const TYPES: readonly { value: ChartTypeDto; label: string }[] = [
  { value: 'bar', label: 'Столбцы' },
  { value: 'stacked_bar', label: 'С накоплением' },
  { value: 'hbar', label: 'Полосы' },
  { value: 'line', label: 'Линия' },
  { value: 'area', label: 'Область' },
  { value: 'donut', label: 'Кольцо' },
  { value: 'table', label: 'Таблица' },
  { value: 'kpi', label: 'Число' }
];

export const METRICS: readonly { value: ChartMetricDto; label: string }[] = [
  { value: 'spent', label: 'Все траты' },
  { value: 'expenses', label: 'Расходы без сбережений' },
  { value: 'income', label: 'Доходы' },
  { value: 'savings', label: 'Сбережения' },
  { value: 'free', label: 'Свободный остаток' },
  { value: 'free_cum', label: 'Остаток накопительно' },
  { value: 'savings_cum', label: 'Сбережения накопительно' },
  { value: 'savings_rate', label: 'Норма сбережений' },
  { value: 'limit_usage', label: 'Использование лимита' },
  { value: 'count', label: 'Количество трат' },
  { value: 'avg_ticket', label: 'Средняя трата' }
];

export const GROUPS: readonly { value: ChartGroupByDto; label: string }[] = [
  { value: 'month', label: 'Месяцам' },
  { value: 'category', label: 'Категориям' },
  { value: 'kind', label: 'Типам' },
  { value: 'status', label: 'Статусам' },
  { value: 'tag', label: 'Тегам' },
  { value: 'none', label: 'Ничему (итог)' }
];

/** `none` — без серий. */
export const SERIES: readonly { value: ChartSeriesByDto | 'none'; label: string }[] = [
  { value: 'none', label: 'Без серий' },
  { value: 'status', label: 'По статусам' },
  { value: 'kind', label: 'По типам' },
  { value: 'category', label: 'По категориям' },
  { value: 'metric', label: 'По показателям' }
];

export type PresetValue = 'ytd' | 'last12' | 'current_month' | 'all' | 'range';

export const PERIODS: readonly { value: PresetValue; label: string }[] = [
  { value: 'ytd', label: 'С начала года' },
  { value: 'last12', label: '12 месяцев' },
  { value: 'current_month', label: 'Текущий месяц' },
  { value: 'all', label: 'Всё время' },
  { value: 'range', label: 'Свой период' }
];

export const SORTS = [
  { value: 'natural', label: 'По порядку' },
  { value: 'desc', label: 'По убыванию' },
  { value: 'asc', label: 'По возрастанию' }
] as const;

export const TOP_N = [
  { value: '0', label: 'Все' },
  { value: '5', label: 'Топ 5' },
  { value: '10', label: 'Топ 10' },
  { value: '20', label: 'Топ 20' }
] as const;

export type BoolOption = 'showLimit' | 'cumulative' | 'comparePrevPeriod' | 'percent';

export const BOOL_OPTIONS: readonly { value: BoolOption; label: string }[] = [
  { value: 'showLimit', label: 'Линия лимита' },
  { value: 'cumulative', label: 'Накопительно' },
  { value: 'comparePrevPeriod', label: 'Сравнить с прошлым периодом' },
  { value: 'percent', label: 'Доли в процентах' }
];

/** Показатели одной единицы, из которых собираются серии «по показателям». */
const DEFAULT_SERIES_METRICS: readonly ChartMetricDto[] = ['income', 'expenses'];

export function withSeries(spec: ChartSpecDto, series: ChartSeriesByDto | 'none'): ChartSpecDto {
  if (series === 'none') return { ...spec, seriesBy: null, metrics: [] };
  if (series !== 'metric') return { ...spec, seriesBy: series, metrics: [] };
  const metrics = spec.metrics.length >= 2 ? spec.metrics : [...DEFAULT_SERIES_METRICS];
  return { ...spec, seriesBy: 'metric', metrics, metric: metrics[0] ?? spec.metric };
}

export function withMetric(spec: ChartSpecDto, metric: ChartMetricDto): ChartSpecDto {
  return { ...spec, metric };
}

/** Один вариант описания: какое значение какого поля он проверяет. */
export type Variant = { id: string; spec: ChartSpecDto };

/**
 * Копии описания с каждым значением каждого переключаемого поля. Бэкенд проверяет их одним
 * вызовом, и конструктор отключает значения, которые сделают описание недопустимым.
 */
export function variants(spec: ChartSpecDto): Variant[] {
  const out: Variant[] = [];
  for (const t of TYPES) out.push({ id: `type:${t.value}`, spec: { ...spec, type: t.value } });
  if (spec.seriesBy !== 'metric') {
    for (const m of METRICS) out.push({ id: `metric:${m.value}`, spec: withMetric(spec, m.value) });
  }
  for (const g of GROUPS) out.push({ id: `group:${g.value}`, spec: { ...spec, groupBy: g.value } });
  for (const s of SERIES) out.push({ id: `series:${s.value}`, spec: withSeries(spec, s.value) });
  for (const o of BOOL_OPTIONS) {
    out.push({
      id: `option:${o.value}`,
      spec: { ...spec, options: { ...spec.options, [o.value]: !spec.options[o.value] } }
    });
  }
  return out;
}

/**
 * Причина, по которой значение недоступно. Если уже само описание недопустимо, вариант,
 * упирающийся в ту же причину, не виноват: значение остаётся доступным.
 */
export function unavailable(
  reasons: Readonly<Record<string, string | null>>,
  current: string | null,
  id: string
): string | null {
  const reason = reasons[id] ?? null;
  return reason !== null && reason !== current ? reason : null;
}

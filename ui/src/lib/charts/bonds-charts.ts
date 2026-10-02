import type {
  BondsMonthDto,
  ChartDataDto,
  ChartSpecDto,
  ForecastScenarioDto
} from '$lib/api/bindings';
import { formatMonth, shiftMonth } from '$lib/format';

/** Данные графиков страницы «Облигации» из ответа `bonds_projection`: только раскладка по осям. */

export const SCENARIO_NAMES = {
  a: 'A · цель нормы',
  b: 'B · норма и экономия',
  c: 'C · как сейчас'
} as const;

/** Копейки → рубли для оси графика (значения `ChartData` — в рублях). */
const rub = (kopecks: number): number => kopecks / 100;

export function factChart(months: readonly BondsMonthDto[]): ChartDataDto {
  return {
    categories: months.map((m) => formatMonth(m.month, 'short')),
    categoryTokens: months.map((m) => `month:${m.month}`),
    series: [
      { name: 'Баланс', color: 'bonds.balance', values: months.map((m) => rub(m.balance)) },
      { name: 'Внесено', color: 'bonds.deposited', values: months.map((m) => rub(m.deposited)) }
    ],
    referenceLines: [],
    totals: null,
    unit: 'rub'
  };
}

/** Прогноз начинается с января года, следующего за `year`; подписи — «Янв 2027». */
export function forecastChart(
  year: number,
  scenarios: readonly ForecastScenarioDto[]
): ChartDataDto {
  const first = `${String(year + 1)}-01`;
  const length = scenarios[0]?.balances.length ?? 0;
  const months = Array.from({ length }, (_, i) => shiftMonth(first, i));
  return {
    categories: months.map((m) => `${formatMonth(m, 'short').slice(0, 3)} ${m.slice(0, 4)}`),
    categoryTokens: months.map(() => ''),
    series: scenarios.map((s) => ({
      name: SCENARIO_NAMES[s.scenario],
      color: `scenario.${s.scenario}.forecast`,
      values: s.balances.map(rub)
    })),
    referenceLines: [],
    totals: null,
    unit: 'rub'
  };
}

/** Описание линейного графика для `Chart`: данные страница передаёт готовыми. */
export function lineSpec(title: string): ChartSpecDto {
  return {
    version: 1,
    title,
    type: 'line',
    metric: 'free_cum',
    groupBy: 'month',
    seriesBy: null,
    metrics: [],
    period: { preset: 'ytd' },
    filter: '',
    options: {
      showLimit: false,
      topN: null,
      sort: 'natural',
      cumulative: false,
      comparePrevPeriod: false,
      percent: false
    }
  };
}

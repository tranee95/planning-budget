import type { ChartSpecDto } from '$lib/api/bindings';

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

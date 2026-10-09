import type { ChartDataDto } from '$lib/api/bindings';

/** Состояние данных одной карточки графика. */
export type CardData =
  | { status: 'loading' }
  | { status: 'ready'; data: ChartDataDto }
  | { status: 'error'; message: string };

/** График нечего рисовать: нет категорий, либо все значения нулевые или пропущены и нет опорных линий. */
export function isEmptyChart(data: ChartDataDto): boolean {
  if (data.categories.length === 0) return true;
  if (data.referenceLines.length > 0) return false;
  return data.series.every((series) =>
    series.values.every((value) => value === null || value === 0)
  );
}

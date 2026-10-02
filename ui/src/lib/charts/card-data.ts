import type { ChartDataDto } from '$lib/api/bindings';

/** Состояние данных одной карточки графика. */
export type CardData =
  | { status: 'loading' }
  | { status: 'ready'; data: ChartDataDto }
  | { status: 'error'; message: string };

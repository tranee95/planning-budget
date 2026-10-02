import type { ChartUnitDto } from '$lib/api/bindings';
import { formatMoney, formatPercent } from '$lib/format';

const count = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 0 });
const compact = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 1 });
const NBSP = ' ';

/** Значение графика в единицах оси: рубли (не копейки), проценты (12,6, а не 0,126), штуки. */
export function formatValue(value: number, unit: ChartUnitDto): string {
  switch (unit) {
    case 'rub':
      return formatMoney(Math.round(value * 100));
    case 'percent':
      return formatPercent(Math.round(value * 100));
    case 'count':
      return count.format(value).replace(/\s/g, NBSP);
  }
}

/** Подпись на оси: «12 тыс.», «1,2 млн» — короткая, чтобы не отъедать место у графика. */
export function formatAxis(value: number, unit: ChartUnitDto): string {
  if (unit === 'percent') return `${compact.format(value)}%`;
  const abs = Math.abs(value);
  if (unit === 'rub' && abs >= 1_000_000) return `${compact.format(value / 1_000_000)}${NBSP}млн`;
  if (unit === 'rub' && abs >= 1000) return `${compact.format(value / 1000)}${NBSP}тыс.`;
  return count.format(value).replace(/\s/g, NBSP);
}

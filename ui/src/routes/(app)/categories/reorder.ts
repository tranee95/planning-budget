/** Вертикальные границы строки списка на экране. */
export type RowBounds = { top: number; bottom: number };

/**
 * Индекс строки под указателем при перетаскивании. Зазор между строками относится к ближайшей,
 * выше первой и ниже последней — крайним строкам: бросить строку «мимо» нельзя.
 */
export function rowIndexAt(y: number, rows: readonly RowBounds[]): number | null {
  let best: number | null = null;
  let bestDistance = Infinity;
  rows.forEach((row, index) => {
    const distance = y < row.top ? row.top - y : y > row.bottom ? y - row.bottom : 0;
    if (distance < bestDistance) {
      best = index;
      bestDistance = distance;
    }
  });
  return best;
}

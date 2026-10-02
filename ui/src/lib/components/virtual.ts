export interface RangeInput {
  scrollTop: number;
  viewport: number;
  rowHeight: number;
  count: number;
  overscan: number;
}

/** Полуинтервал `[start, end)` строк, которые нужно отрисовать. */
export function visibleRange(input: RangeInput): { start: number; end: number } {
  const { scrollTop, viewport, rowHeight, count, overscan } = input;
  const first = Math.floor(scrollTop / rowHeight);
  const last = Math.ceil((scrollTop + viewport) / rowHeight);
  return {
    start: Math.max(0, Math.min(first - overscan, count)),
    end: Math.max(0, Math.min(last + overscan, count))
  };
}

/**
 * Активная строка после прокрутки: если она ушла за видимую область, активной становится
 * ближайшая видимая. Иначе `aria-activedescendant` указывал бы на строку, которой нет в DOM.
 */
export function clampActive(input: Omit<RangeInput, 'overscan'> & { active: number }): number {
  const { scrollTop, viewport, rowHeight, count, active } = input;
  if (count === 0) return 0;
  const first = Math.min(Math.ceil(scrollTop / rowHeight), count - 1);
  const last = Math.max(Math.floor((scrollTop + viewport) / rowHeight) - 1, first);
  return Math.min(Math.max(active, first), Math.min(last, count - 1));
}

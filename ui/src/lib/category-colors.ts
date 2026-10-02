/** Палитра цветов категорий: цвет — данные категории, поэтому hex хранится в БД. */
export const CATEGORY_COLORS: readonly string[] = [
  '#3AA567',
  '#5B7FD6',
  '#8C6BD1',
  '#D9893A',
  '#DD5249',
  '#C552A0',
  '#4E8F6E',
  '#E0A73A',
  '#6C7480',
  '#3445D0'
];

/** «14» или «14,5» → базисные пункты (1400, 1450); `null` — не число или вне 0–100 %. */
export function parsePercentBp(text: string): number | null {
  const normalized = text.trim().replace(',', '.').replace(/\s?%$/, '');
  if (!/^\d{1,3}(\.\d{1,2})?$/.test(normalized)) return null;
  const bp = Math.round(Number(normalized) * 100);
  return bp <= 10_000 ? bp : null;
}

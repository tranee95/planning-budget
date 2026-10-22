import { describe, expect, it } from 'vitest';
import { niceTop } from './scale';

describe('niceTop', () => {
  it('держит запас небольшим: 105 000 при 4 делениях → 120 000, а не 200 000', () => {
    expect(niceTop(105_000, 4)).toBe(120_000);
  });
  it('максимум не упирается в верх: 30 000 при 5 делениях → 40 000', () => {
    expect(niceTop(30_000, 5)).toBe(40_000);
  });
  it('верх выше максимума с запасом', () => {
    for (const peak of [1, 7, 999, 12_345, 100_000, 2_345_678]) {
      expect(niceTop(peak, 5)).toBeGreaterThan(peak);
      expect(niceTop(peak, 4)).toBeGreaterThan(peak);
    }
  });
  it('пустые данные дают 0', () => {
    expect(niceTop(0, 4)).toBe(0);
    expect(niceTop(-5, 4)).toBe(0);
  });
});

import { describe, expect, it } from 'vitest';
import { MonthStore } from './month.svelte';

describe('MonthStore', () => {
  it('стартует с переданного месяца', () => {
    expect(new MonthStore('2026-09').current).toBe('2026-09');
  });

  it('запоминает направление перехода для анимации', () => {
    const store = new MonthStore('2026-09');
    store.shift(1);
    expect(store.current).toBe('2026-10');
    expect(store.direction).toBe(1);
    store.shift(-1);
    store.shift(-1);
    expect(store.current).toBe('2026-08');
    expect(store.direction).toBe(-1);
  });

  it('«сегодня» возвращает к текущему месяцу', () => {
    const store = new MonthStore('2026-01', () => '2026-10');
    store.today();
    expect(store.current).toBe('2026-10');
    expect(store.direction).toBe(1);
  });
});

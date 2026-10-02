import { describe, expect, it } from 'vitest';
import { clampActive, visibleRange } from './virtual';

describe('visibleRange', () => {
  it('показывает строки в окне плюс запас', () => {
    expect(
      visibleRange({ scrollTop: 0, viewport: 200, rowHeight: 40, count: 1000, overscan: 2 })
    ).toEqual({
      start: 0,
      end: 7
    });
    expect(
      visibleRange({ scrollTop: 400, viewport: 200, rowHeight: 40, count: 1000, overscan: 2 })
    ).toEqual({
      start: 8,
      end: 17
    });
  });

  it('не выходит за границы списка', () => {
    expect(
      visibleRange({ scrollTop: 39_600, viewport: 400, rowHeight: 40, count: 1000, overscan: 5 })
        .end
    ).toBe(1000);
    expect(
      visibleRange({ scrollTop: 0, viewport: 400, rowHeight: 40, count: 3, overscan: 5 })
    ).toEqual({
      start: 0,
      end: 3
    });
  });

  it('пустой список', () => {
    expect(
      visibleRange({ scrollTop: 0, viewport: 400, rowHeight: 40, count: 0, overscan: 5 })
    ).toEqual({
      start: 0,
      end: 0
    });
  });
});

describe('clampActive', () => {
  const base = { scrollTop: 0, viewport: 440, rowHeight: 44, count: 100 };

  it('видимая строка остаётся активной', () => {
    expect(clampActive({ ...base, active: 3 })).toBe(3);
  });

  it('прокрутка вниз колесом переносит активную строку на первую видимую', () => {
    expect(clampActive({ ...base, scrollTop: 44 * 50, active: 3 })).toBe(50);
  });

  it('прокрутка вверх переносит её на последнюю видимую', () => {
    expect(clampActive({ ...base, scrollTop: 0, active: 80 })).toBe(9);
  });

  it('пустой список и короткий список', () => {
    expect(clampActive({ ...base, count: 0, active: 5 })).toBe(0);
    expect(clampActive({ ...base, count: 4, active: 9 })).toBe(3);
  });
});

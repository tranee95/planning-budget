import { describe, expect, it } from 'vitest';
import { placeList } from './popover';

const base = { viewport: 600, desired: 240 };

describe('placeList', () => {
  it('открывает вниз, если места хватает', () => {
    expect(placeList({ ...base, triggerTop: 100, triggerBottom: 140 })).toEqual({
      up: false,
      maxHeight: 240
    });
  });

  it('открывает вверх у нижнего края окна', () => {
    const r = placeList({ ...base, triggerTop: 540, triggerBottom: 580 });
    expect(r.up).toBe(true);
    expect(r.maxHeight).toBe(240);
  });

  it('ограничивает высоту, если места мало и сверху, и снизу', () => {
    const r = placeList({ viewport: 300, desired: 240, triggerTop: 120, triggerBottom: 160 });
    expect(r.maxHeight).toBeLessThan(240);
    expect(r.maxHeight).toBeGreaterThanOrEqual(96);
  });

  it('выбирает сторону, где места больше', () => {
    expect(placeList({ viewport: 400, desired: 240, triggerTop: 250, triggerBottom: 290 }).up).toBe(
      true
    );
    expect(placeList({ viewport: 400, desired: 240, triggerTop: 50, triggerBottom: 90 }).up).toBe(
      false
    );
  });
});

import { clampShift } from './popover';

describe('clampShift', () => {
  it('в середине окна сдвиг не нужен', () => {
    expect(clampShift({ center: 500, width: 100, viewport: 960 })).toBe(0);
  });

  it('у правого края сдвигает влево на выступающую часть и поле', () => {
    // right = 940 + 50 = 990, допустимо до 952
    expect(clampShift({ center: 940, width: 100, viewport: 960 })).toBe(-38);
  });

  it('у левого края сдвигает вправо', () => {
    expect(clampShift({ center: 10, width: 100, viewport: 960 })).toBe(48);
  });
});

import { describe, expect, it } from 'vitest';
import { COLUMNS, bottom, compact, place, sameLayout, settle, sizeOf, type Box } from './layout';

const box = (id: number, x: number, y: number, w = 6, h = 4): Box => ({ id, x, y, w, h });
const at = (boxes: Box[], id: number): Box | undefined => boxes.find((b) => b.id === id);

describe('compact', () => {
  it('lifts cards into gaps without changing their columns', () => {
    const result = compact([box(1, 0, 0), box(2, 0, 9), box(3, 6, 5)]);
    expect(at(result, 2)).toMatchObject({ x: 0, y: 4 });
    expect(at(result, 3)).toMatchObject({ x: 6, y: 0 });
  });

  it('keeps cards that already touch', () => {
    const start = [box(1, 0, 0), box(2, 6, 0), box(3, 0, 4, 12)];
    expect(sameLayout(compact(start), start)).toBe(true);
  });
});

describe('place', () => {
  it('pins the dragged card and pushes the others below it', () => {
    const start = [box(1, 0, 0), box(2, 6, 0), box(3, 0, 4)];
    const result = place(start, 3, { x: 0, y: 0 });
    expect(at(result, 3)).toMatchObject({ x: 0, y: 0 });
    expect(at(result, 1)).toMatchObject({ x: 0, y: 4 });
    expect(at(result, 2)).toMatchObject({ x: 6, y: 0 });
  });

  it('never leaves overlaps or goes outside the grid', () => {
    const start = [box(1, 0, 0), box(2, 6, 0), box(3, 0, 4, 12, 2)];
    const result = place(start, 2, { x: 40, y: -5 });
    expect(at(result, 2)).toMatchObject({ x: COLUMNS - 6, y: 0 });
    for (const a of result) {
      expect(a.x + a.w).toBeLessThanOrEqual(COLUMNS);
      for (const b of result) {
        if (a.id === b.id) continue;
        const overlap = a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
        expect(overlap).toBe(false);
      }
    }
  });

  it('resizes through the same path', () => {
    const start = [box(1, 0, 0), box(2, 6, 0)];
    const result = place(start, 1, { w: 12 });
    expect(at(result, 1)).toMatchObject({ x: 0, w: 12 });
    expect(at(result, 2)).toMatchObject({ y: 4 });
  });
});

describe('settle', () => {
  it('drops a card released in empty space to the first free row', () => {
    const start = [box(1, 0, 0), box(2, 6, 0)];
    const result = settle(start, 2, { x: 6, y: 30 });
    expect(at(result, 2)).toMatchObject({ x: 6, y: 0 });
  });

  it('ignores unknown ids', () => {
    const start = [box(1, 0, 0)];
    expect(sameLayout(place(start, 9, { x: 3 }), start)).toBe(true);
  });
});

describe('helpers', () => {
  it('finds the free row below everything', () => {
    expect(bottom([])).toBe(0);
    expect(bottom([box(1, 0, 0), box(2, 6, 3, 6, 5)])).toBe(8);
  });

  it('names the S/M/L presets by width', () => {
    expect(sizeOf(box(1, 0, 0, 4))).toBe('S');
    expect(sizeOf(box(1, 0, 0, 6))).toBe('M');
    expect(sizeOf(box(1, 0, 0, 12))).toBe('L');
    expect(sizeOf(box(1, 0, 0, 5))).toBeNull();
  });
});

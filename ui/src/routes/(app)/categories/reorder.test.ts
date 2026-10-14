import { expect, it } from 'vitest';
import { rowIndexAt } from './reorder';

const rows = [
  { top: 100, bottom: 148 },
  { top: 150, bottom: 198 },
  { top: 200, bottom: 248 }
];

it('rowIndexAt: точка внутри строки даёт её индекс', () => {
  expect(rowIndexAt(120, rows)).toBe(0);
  expect(rowIndexAt(175, rows)).toBe(1);
  expect(rowIndexAt(240, rows)).toBe(2);
});

it('rowIndexAt: зазор между строками относится к ближайшей', () => {
  expect(rowIndexAt(149, rows)).toBe(0);
  expect(rowIndexAt(151, rows)).toBe(1);
});

it('rowIndexAt: выше первой и ниже последней — крайние строки', () => {
  expect(rowIndexAt(-50, rows)).toBe(0);
  expect(rowIndexAt(900, rows)).toBe(2);
});

it('rowIndexAt: пустой список — null', () => {
  expect(rowIndexAt(10, [])).toBeNull();
});

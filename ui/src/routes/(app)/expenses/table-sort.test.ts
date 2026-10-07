import { expect, it } from 'vitest';
import type { TransactionDto } from '$lib/api/bindings';
import { sortRows } from './table-sort';

const row = (id: number, patch: Partial<TransactionDto>): TransactionDto => ({
  id,
  month: '2026-09',
  date: null,
  categoryId: 1,
  title: 'Запись',
  amount: 100,
  status: 'paid',
  comment: null,
  tagIds: [],
  ...patch
});

const ids = (rows: readonly TransactionDto[]): number[] => rows.map((r) => r.id);

it('без сортировки возвращает строки как есть', () => {
  const rows = [row(2, { amount: 5 }), row(1, { amount: 9 })];
  expect(sortRows(rows, null, new Map())).toBe(rows);
});

it('сортирует по сумме в обе стороны и не меняет исходный массив', () => {
  const rows = [row(1, { amount: 300 }), row(2, { amount: 100 }), row(3, { amount: 200 })];
  expect(ids(sortRows(rows, { column: 'amount', descending: false }, new Map()))).toEqual([
    2, 3, 1
  ]);
  expect(ids(sortRows(rows, { column: 'amount', descending: true }, new Map()))).toEqual([1, 3, 2]);
  expect(ids(rows)).toEqual([1, 2, 3]);
});

it('строки без даты уходят в конец при сортировке по дате', () => {
  const rows = [
    row(1, { date: null }),
    row(2, { date: '2026-09-02' }),
    row(3, { date: '2026-09-01' })
  ];
  expect(ids(sortRows(rows, { column: 'date', descending: false }, new Map()))).toEqual([3, 2, 1]);
});

it('категории сравниваются по русскому алфавиту, статусы — по порядку из дизайн-системы', () => {
  const names = new Map([
    [1, 'Ёлки'],
    [2, 'Аптека']
  ]);
  const rows = [
    row(1, { categoryId: 1, status: 'planned' }),
    row(2, { categoryId: 2, status: 'paid' })
  ];
  expect(ids(sortRows(rows, { column: 'category', descending: false }, names))).toEqual([2, 1]);
  expect(ids(sortRows(rows, { column: 'status', descending: false }, names))).toEqual([2, 1]);
});

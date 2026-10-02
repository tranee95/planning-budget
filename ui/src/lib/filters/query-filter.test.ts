import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { FILTER_DEBOUNCE_MS, QueryFilter } from './query-filter.svelte';

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

test('ввод применяется после паузы, последний текст побеждает', () => {
  const filter = new QueryFilter();
  filter.type('л');
  filter.type('лента ');
  expect(filter.applied).toBe('');
  vi.advanceTimersByTime(FILTER_DEBOUNCE_MS);
  expect(filter.applied).toBe('лента');
  expect(filter.active).toBe(true);
  expect(filter.text).toBe('лента ');
});

test('пустой ввод снимает фильтр сразу', () => {
  const filter = new QueryFilter();
  filter.set('лента');
  filter.type('  ');
  expect(filter.applied).toBe('');
  expect(filter.active).toBe(false);
});

test('set применяет сразу и отменяет отложенный ввод', () => {
  const filter = new QueryFilter();
  filter.type('магнит');
  filter.set('сумма>5000');
  vi.advanceTimersByTime(FILTER_DEBOUNCE_MS * 2);
  expect(filter.applied).toBe('сумма>5000');
});

test('clear сбрасывает текст, запрос и итоги', () => {
  const filter = new QueryFilter();
  filter.set('лента');
  filter.meta = { query: 'лента', spans: [], hints: [], total: 1, sum: 100, truncated: false };
  filter.clear();
  expect([filter.text, filter.applied, filter.meta]).toEqual(['', '', null]);
});

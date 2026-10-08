import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { SearchResultDto } from '$lib/api/bindings';
import { buildGroups } from './entries';
import { PaletteSearch, recentQueries, SEARCH_DEBOUNCE_MS } from './search.svelte';

function result(requestId: number, title = 'Лента'): SearchResultDto {
  return {
    requestId,
    spans: [{ start: 0, end: 5, kind: 'text', negated: false }],
    hints: [],
    transactions: {
      total: 9,
      sum: 11_888_100,
      items: [
        {
          id: 1,
          title,
          categoryId: 2,
          category: 'Продукты',
          month: '2026-09',
          date: '2026-09-12',
          amount: 1_320_000,
          status: 'paid'
        }
      ]
    },
    incomes: { total: 0, sum: 0, items: [] },
    categories: [],
    months: []
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  recentQueries.reset();
});
afterEach(() => {
  vi.useRealTimers();
});

test('быстрый ввод даёт один запрос с последним текстом', async () => {
  const run = vi.fn((_q: string, id: number) => Promise.resolve(result(id)));
  const search = new PaletteSearch(run);
  search.set('л');
  search.set('ле');
  search.set('лента');
  expect(run).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
  expect(run).toHaveBeenCalledOnce();
  expect(run.mock.calls[0]?.[0]).toBe('лента');
  expect(search.result?.query).toBe('лента');
  expect(search.pending).toBe(false);
});

test('устаревший ответ не затирает свежий', async () => {
  const resolvers: Array<(r: SearchResultDto) => void> = [];
  const run = vi.fn(
    () =>
      new Promise<SearchResultDto>((resolve) => {
        resolvers.push(resolve);
      })
  );
  const search = new PaletteSearch(run, 10);
  search.set('лента');
  await vi.advanceTimersByTimeAsync(10);
  search.set('магнит');
  await vi.advanceTimersByTimeAsync(10);
  expect(resolvers).toHaveLength(2);

  resolvers[1]?.(result(2, 'Магнит'));
  await vi.advanceTimersByTimeAsync(0);
  resolvers[0]?.(result(1, 'Лента'));
  await vi.advanceTimersByTimeAsync(0);

  expect(search.result?.query).toBe('магнит');
  expect(search.result?.data.transactions.items[0]?.title).toBe('Магнит');
});

test('пустой запрос сбрасывает результат и не ходит в бэкенд', async () => {
  const run = vi.fn((_q: string, id: number) => Promise.resolve(result(id)));
  const search = new PaletteSearch(run, 10);
  search.set('лента');
  await vi.advanceTimersByTimeAsync(10);
  search.set('  ');
  await vi.advanceTimersByTimeAsync(50);
  expect(run).toHaveBeenCalledOnce();
  expect(search.result).toBeNull();
});

test('ошибка поиска оставляет палитру рабочей', async () => {
  const search = new PaletteSearch(() => Promise.reject(new Error('boom')), 10);
  search.set('лента');
  await vi.advanceTimersByTimeAsync(10);
  expect(search.result).toBeNull();
  expect(search.pending).toBe(false);
});

test('недавние: максимум 5, без повторов, свежий первым', () => {
  for (const q of ['a', 'b', 'c', 'd', 'e', 'f', 'c']) recentQueries.remember(q);
  recentQueries.remember('  ');
  expect(recentQueries.items).toEqual(['c', 'f', 'e', 'd', 'b']);
});

test('группы: траты с итогом, затем команды; пустой запрос — недавние и команды', () => {
  const command = { id: 'lock', label: 'Заблокировать', group: 'Сейф', run: vi.fn() };
  const found = buildGroups('лента', result(1), [command], [], []);
  expect(found.map((g) => g.id)).toEqual(['transactions', 'actions']);
  expect(found[1]?.entries[0]).toMatchObject({
    label: 'Показать все траты таблицей (9)',
    target: { type: 'table', screen: 'expenses', query: 'лента' }
  });
  expect(found[0]?.title).toMatch(/^Траты · найдено 9 · 118\s881\s₽$/);
  expect(found[0]?.entries[0]?.meta).toBe('Продукты · 12.09');

  const idle = buildGroups(
    '',
    null,
    [command],
    ['лента статус:план'],
    [
      { id: 3, name: 'Крупное', query: 'сумма>10000', screen: 'expenses' },
      { id: 4, name: 'Зарплата', query: 'зарплата', screen: 'incomes' }
    ]
  );
  expect(idle.map((g) => g.id)).toEqual(['recent', 'saved', 'commands']);
  expect(idle[1]?.entries[0]?.target).toEqual({
    type: 'table',
    screen: 'expenses',
    query: 'сумма>10000'
  });
  expect(idle[1]?.entries[1]?.target).toEqual({
    type: 'table',
    screen: 'incomes',
    query: 'зарплата'
  });
  expect(idle[0]?.entries[0]?.target).toEqual({ type: 'recent', query: 'лента статус:план' });
});

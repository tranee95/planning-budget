import { emit } from '@tauri-apps/api/event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installMocks } from '$lib/api/mock/install';
import {
  COALESCE_MS,
  LoadStamp,
  mergeChanges,
  onDataChanged,
  type DataChange
} from './data-events';

beforeEach(() => {
  installMocks();
});
afterEach(() => {
  clearMocks();
});

const settle = (ms = COALESCE_MS * 3): Promise<void> => new Promise((r) => setTimeout(r, ms));

it('серия событий одной правки даёт один вызов с объединёнными месяцами', async () => {
  const handler = vi.fn();
  const off = onDataChanged(handler);
  await settle(20);
  await emit('data-changed', { scope: 'transactions', months: ['2026-08'] });
  await emit('data-changed', { scope: 'transactions', months: ['2026-09'] });
  await emit('data-changed', { scope: 'categories', months: [] });
  await settle();
  expect(handler).toHaveBeenCalledOnce();
  expect(handler.mock.calls[0]?.[0]).toMatchObject({ scope: 'all', months: [] });
  off();
});

it('несколько подписчиков делят одну подписку, фильтр по области сохраняется', async () => {
  const all = vi.fn();
  const onlyTags = vi.fn();
  const offA = onDataChanged(all);
  const offB = onDataChanged(onlyTags, ['tags']);
  await settle(20);
  await emit('data-changed', { scope: 'transactions', months: ['2026-09'] });
  await settle();
  expect(all).toHaveBeenCalledOnce();
  expect(onlyTags).not.toHaveBeenCalled();
  offA();
  offB();
});

it('после cleanup обработчик не вызывается', async () => {
  const handler = vi.fn();
  const off = onDataChanged(handler);
  await settle(20);
  off();
  await emit('data-changed', { scope: 'tags', months: [] });
  await settle();
  expect(handler).not.toHaveBeenCalled();
});

it('mergeChanges: одна область, месяцы объединяются без дублей', () => {
  const a: DataChange = { scope: 'transactions', months: ['2026-08'], receivedAt: 1 };
  const b: DataChange = { scope: 'transactions', months: ['2026-08', '2026-09'], receivedAt: 2 };
  expect(mergeChanges(a, b)).toEqual({
    scope: 'transactions',
    months: ['2026-08', '2026-09'],
    receivedAt: 2
  });
});

it('LoadStamp: загрузка после изменения делает данные свежими', () => {
  const stamp = new LoadStamp();
  const change: DataChange = { scope: 'transactions', months: [], receivedAt: performance.now() };
  expect(stamp.isFresh(change)).toBe(false);
  stamp.mark();
  expect(stamp.isFresh(change)).toBe(true);
  expect(stamp.isFresh({ ...change, receivedAt: performance.now() + 1000 })).toBe(false);
});

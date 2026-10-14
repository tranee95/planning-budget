import { clearMocks } from '@tauri-apps/api/mocks';
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { events } from '$lib/api/bindings';
import { installMocks } from '$lib/api/mock/install';
import { session } from '$lib/stores/session.svelte';
import { cachedCall, dayKey, ResultCache, resultCache, watchResultCache } from './result-cache';

let cache: ResultCache;

afterEach(() => {
  cache.clear();
});

it('отдаёт то, что положили, пока данные не менялись', () => {
  cache = new ResultCache();
  cache.set('a', { x: 1 }, cache.epoch);
  expect(cache.get('a')).toEqual({ x: 1 });
  expect(cache.get('b')).toBeUndefined();
});

it('после clear всё забыто, номер поколения растёт', () => {
  cache = new ResultCache();
  const before = cache.epoch;
  cache.set('a', 1, before);
  cache.clear();
  expect(cache.get('a')).toBeUndefined();
  expect(cache.epoch).toBe(before + 1);
});

it('ответ, начатый до изменения данных, в кеш не попадает', () => {
  cache = new ResultCache();
  const started = cache.epoch;
  cache.clear();
  cache.set('late', 'old data', started);
  expect(cache.get('late')).toBeUndefined();
});

it('ограничение размера: самые старые записи вытесняются', () => {
  cache = new ResultCache(2);
  cache.set('a', 1, cache.epoch);
  cache.set('b', 2, cache.epoch);
  cache.set('c', 3, cache.epoch);
  expect(cache.get('a')).toBeUndefined();
  expect(cache.get('b')).toBe(2);
  expect(cache.get('c')).toBe(3);
});

it('cachedCall: второй вызов с тем же ключом не ходит за данными', async () => {
  cache = new ResultCache();
  resultCache.clear();
  let calls = 0;
  const load = (): Promise<number> => {
    calls += 1;
    return Promise.resolve(calls);
  };
  expect(await cachedCall('k', load)).toBe(1);
  expect(await cachedCall('k', load)).toBe(1);
  expect(calls).toBe(1);
  resultCache.clear();
  expect(await cachedCall('k', load)).toBe(2);
});

it('cachedCall: сбой не кешируется, изменение данных во время запроса тоже', async () => {
  resultCache.clear();
  await expect(cachedCall('f', () => Promise.reject(new Error('boom')))).rejects.toThrow('boom');
  expect(resultCache.get('f')).toBeUndefined();
  const value = await cachedCall('slow', () => {
    resultCache.clear();
    return Promise.resolve('before change');
  });
  expect(value).toBe('before change');
  expect(resultCache.get('slow')).toBeUndefined();
});

it('dayKey меняется со сменой даты', () => {
  expect(dayKey(new Date('2026-10-14T10:00:00'))).not.toBe(dayKey(new Date('2026-10-15T10:00:00')));
});

// ---- подписка на изменения и блокировку ----
// Подписка одна на всё приложение: мок событий ставится один раз на группу тестов.

describe('подписка', () => {
  beforeAll(async () => {
    installMocks();
    watchResultCache();
    await new Promise((r) => setTimeout(r, 0));
  });

  afterAll(() => {
    clearMocks();
  });

  it('data-changed очищает кеш, блокировка тоже', async () => {
    resultCache.set('k', 'v', resultCache.epoch);
    expect(resultCache.get('k')).toBe('v');
    await events.dataChanged.emit({ scope: 'transactions', months: ['2026-09'] });
    await vi.waitFor(() => {
      expect(resultCache.get('k')).toBeUndefined();
    });
    resultCache.set('k2', 'v2', resultCache.epoch);
    session.lockedFromBackend();
    expect(resultCache.get('k2')).toBeUndefined();
  });

  it('ответ, начатый до события data-changed, в кеш не попадает', async () => {
    const value = await cachedCall('race', async () => {
      await events.dataChanged.emit({ scope: 'plan', months: [] });
      await vi.waitFor(() => {
        expect(resultCache.epoch).toBeGreaterThan(0);
      });
      await new Promise((r) => setTimeout(r, 20));
      return 'stale';
    });
    expect(value).toBe('stale');
    expect(resultCache.get('race')).toBeUndefined();
  });
});

it('отключённый кеш ничего не запоминает до повторного включения', () => {
  const local = new ResultCache();
  local.disable();
  local.set('a', 1, local.epoch);
  expect(local.get('a')).toBeUndefined();
  local.enable();
  local.set('a', 1, local.epoch);
  expect(local.get('a')).toBe(1);
});

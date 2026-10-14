import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { UpdateStatusDto } from '$lib/api/bindings';
import { ApiError } from '$lib/api/call';
import { updatesApi } from '$lib/api/updates';
import { toasts } from './toasts.svelte';
import { updates } from './updates.svelte';

const status = (over: Partial<UpdateStatusDto>): UpdateStatusDto => ({
  state: 'upToDate',
  currentVersion: '0.6.16',
  version: null,
  notes: null,
  ...over
});
const available = status({ state: 'available', version: '0.7.0', notes: 'Исправления.' });

beforeEach(() => {
  updates.reset();
  toasts.items = [];
});

afterEach(() => {
  vi.restoreAllMocks();
});

it('автопроверка: есть новая версия — предложение показывается, «Позже» его прячет', async () => {
  vi.spyOn(updatesApi, 'check').mockResolvedValue(available);
  await updates.autoCheck();
  expect(updates.offer?.version).toBe('0.7.0');
  updates.dismiss();
  expect(updates.offer).toBeNull();
  expect(updates.status?.state).toBe('available');
});

it('автопроверка идёт один раз за запуск и с manual = false', async () => {
  const check = vi.spyOn(updatesApi, 'check').mockResolvedValue(status({}));
  await updates.autoCheck();
  await updates.autoCheck();
  expect(check).toHaveBeenCalledTimes(1);
  expect(check).toHaveBeenCalledWith(false);
});

it('«выключено» и «пропущено» не затирают прежний итог', async () => {
  vi.spyOn(updatesApi, 'check').mockResolvedValue(status({ state: 'disabled' }));
  await updates.autoCheck();
  expect(updates.status).toBeNull();
});

it('сбой автопроверки молча игнорируется', async () => {
  vi.spyOn(updatesApi, 'check').mockRejectedValue(new Error('offline'));
  await updates.autoCheck();
  expect(updates.status).toBeNull();
  expect(toasts.items).toHaveLength(0);
});

it('ручная проверка идёт с manual = true и снова показывает предложение', async () => {
  const check = vi.spyOn(updatesApi, 'check').mockResolvedValue(available);
  updates.dismissed = true;
  await updates.checkNow();
  expect(check).toHaveBeenCalledWith(true);
  expect(updates.offer?.version).toBe('0.7.0');
  expect(updates.checking).toBe(false);
});

it('установка: ошибка показывается тостом, кнопка снова доступна', async () => {
  vi.spyOn(updatesApi, 'install').mockRejectedValue(
    new ApiError({ code: 'Io', messageKey: 'errors.io.update' })
  );
  await updates.install();
  expect(toasts.items.at(-1)?.kind).toBe('error');
  expect(toasts.items.at(-1)?.message).toContain('обновление');
  expect(updates.installing).toBe(false);
});

it('установка не стартует дважды', async () => {
  let release: (value: null) => void = () => undefined;
  const install = vi.spyOn(updatesApi, 'install').mockReturnValue(
    new Promise((resolve) => {
      release = resolve;
    })
  );
  const first = updates.install();
  await updates.install();
  release(null);
  await first;
  expect(install).toHaveBeenCalledTimes(1);
});

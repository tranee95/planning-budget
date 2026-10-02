import { emit } from '@tauri-apps/api/event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installMocks } from '$lib/api/mock/install';
import { shortcuts } from '$lib/shortcuts';
import { affectsMonth, connectScreen } from './connect-screen';

beforeEach(() => {
  installMocks();
});
afterEach(() => {
  clearMocks();
});

it('affectsMonth: пустой список — все месяцы, иначе только перечисленные', () => {
  expect(affectsMonth([], '2026-09')).toBe(true);
  expect(affectsMonth(['2026-09'], '2026-09')).toBe(true);
  expect(affectsMonth(['2026-08'], '2026-09')).toBe(false);
  expect(affectsMonth([], '')).toBe(false);
});

it('connectScreen: клавиша работает до cleanup и не работает после', () => {
  const run = vi.fn();
  const off = connectScreen({
    shortcut: { id: 'test.add', code: 'KeyN', label: 'Добавить', run },
    month: () => '2026-09',
    reload: vi.fn()
  });
  const press = (): void => {
    shortcuts.handle(new KeyboardEvent('keydown', { code: 'KeyN', cancelable: true }));
  };
  press();
  expect(run).toHaveBeenCalledOnce();
  off();
  press();
  expect(run).toHaveBeenCalledOnce();
});

it('connectScreen: data-changed перезагружает экран, только если затронут его месяц', async () => {
  const reload = vi.fn();
  const off = connectScreen({
    shortcut: { id: 'test.reload', code: 'KeyJ', label: 'Тест', run: vi.fn() },
    month: () => '2026-09',
    reload
  });
  // Подписка на событие оформляется асинхронно: даём ей зарегистрироваться до первого emit.
  await new Promise((resolve) => setTimeout(resolve, 20));
  await emit('data-changed', { scope: 'transactions', months: ['2026-08'] });
  await emit('data-changed', { scope: 'transactions', months: ['2026-09'] });
  await vi.waitFor(() => {
    expect(reload).toHaveBeenCalledOnce();
  });
  off();
});

it('connectScreen: список без привязки к месяцу перезагружается при любом изменении', async () => {
  const reload = vi.fn();
  const off = connectScreen({
    shortcut: { id: 'test.any', code: 'KeyH', label: 'Тест', run: vi.fn() },
    month: () => '2026-09',
    anyMonth: () => true,
    reload
  });
  await new Promise((resolve) => setTimeout(resolve, 20));
  await emit('data-changed', { scope: 'transactions', months: ['2020-01'] });
  await vi.waitFor(() => {
    expect(reload).toHaveBeenCalledOnce();
  });
  off();
});

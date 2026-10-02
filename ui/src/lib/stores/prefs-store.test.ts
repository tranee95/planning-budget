import { beforeEach, expect, it, vi } from 'vitest';

const listeners: ((e: { payload: { dark: boolean } }) => void)[] = [];
const listen = vi.fn((cb: (e: { payload: { dark: boolean } }) => void) => {
  listeners.push(cb);
  return Promise.resolve(() => undefined);
});
const prefsApi = {
  get: vi.fn(() =>
    Promise.resolve({
      theme: 'system',
      locale: 'ru-RU',
      uiScale: 100,
      reducedMotion: 'system',
      autolockMinutes: 5,
      showTips: true
    })
  ),
  set: vi.fn(),
  systemDark: vi.fn(() => Promise.resolve(false))
};

vi.mock('$lib/api/bindings', () => ({ events: { systemThemeChanged: { listen } } }));
vi.mock('$lib/api/prefs', () => ({ prefsApi }));

beforeEach(() => {
  vi.resetModules(); // у каждого теста своё хранилище: подписка делается один раз за его жизнь
  listeners.length = 0;
  listen.mockClear();
  delete document.documentElement.dataset.theme;
});

it('подписка на смену темы ОС одна, сколько бы раз ни вызвали load()', async () => {
  const { prefs } = await import('./prefs.svelte');
  await prefs.load();
  await prefs.load();
  expect(listen).toHaveBeenCalledTimes(1);
});

it('событие меняет «системную» тему на лету', async () => {
  const { prefs } = await import('./prefs.svelte');
  await prefs.load();
  expect(document.documentElement.dataset.theme).toBe('light');
  listeners[0]?.({ payload: { dark: true } });
  expect(document.documentElement.dataset.theme).toBe('dark');
});

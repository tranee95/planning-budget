import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { copySecret } from './clipboard';

let content: string;
let readable: boolean;

beforeEach(() => {
  vi.useFakeTimers();
  content = '';
  readable = true;
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: vi.fn((text: string) => {
        content = text;
        return Promise.resolve();
      }),
      readText: vi.fn(() =>
        readable ? Promise.resolve(content) : Promise.reject(new Error('denied'))
      )
    }
  });
});

afterEach(() => {
  vi.useRealTimers();
});

test('секрет стирается из буфера по таймеру, если он всё ещё там', async () => {
  const handle = await copySecret('SECRET', 30_000);
  expect(content).toBe('SECRET');
  await vi.advanceTimersByTimeAsync(29_999);
  expect(content).toBe('SECRET');
  await vi.advanceTimersByTimeAsync(1);
  expect(content).toBe('');
  expect(handle.cleared).toBe(true);
});

test('чужое содержимое буфера не трогается', async () => {
  await copySecret('SECRET', 30_000);
  content = 'другое, скопированное позже';
  await vi.advanceTimersByTimeAsync(30_000);
  expect(content).toBe('другое, скопированное позже');
});

test('буфер нельзя прочитать — ничего не стираем вслепую', async () => {
  readable = false;
  const handle = await copySecret('SECRET', 30_000);
  await vi.advanceTimersByTimeAsync(30_000);
  expect(content).toBe('SECRET');
  expect(handle.cleared).toBe(false);
});

test('clearNow стирает сразу и отменяет таймер', async () => {
  const handle = await copySecret('SECRET', 30_000);
  await handle.clearNow();
  expect(content).toBe('');
  content = 'новое';
  await vi.advanceTimersByTimeAsync(30_000);
  expect(content).toBe('новое');
});

test('ошибка записи пробрасывается', async () => {
  const write = vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValueOnce(new Error('no'));
  await expect(copySecret('SECRET', 1000)).rejects.toThrow();
  expect(write).toHaveBeenCalledOnce();
});

test('буфер недоступен без фокуса: стирание повторяется, когда окно получает фокус', async () => {
  readable = false;
  const handle = await copySecret('SECRET', 30_000);
  await vi.advanceTimersByTimeAsync(30_000);
  expect(content).toBe('SECRET');
  readable = true;
  window.dispatchEvent(new Event('focus'));
  await vi.advanceTimersByTimeAsync(0);
  expect(content).toBe('');
  expect(handle.cleared).toBe(true);
});

test('новое копирование отменяет ожидание фокуса прежнего: свежая копия не стирается раньше срока', async () => {
  readable = false;
  await copySecret('SECRET', 30_000);
  await vi.advanceTimersByTimeAsync(30_000);
  // Окно без фокуса: первая копия ждёт фокус. Пользователь копирует тот же код снова.
  readable = true;
  await copySecret('SECRET', 30_000);
  window.dispatchEvent(new Event('focus'));
  await vi.advanceTimersByTimeAsync(0);
  expect(content).toBe('SECRET');
  await vi.advanceTimersByTimeAsync(30_000);
  expect(content).toBe('');
});

test('повторные неудачи не копят слушателей фокуса', async () => {
  readable = false;
  const add = vi.spyOn(window, 'addEventListener');
  const handle = await copySecret('SECRET', 30_000);
  await handle.clearNow();
  await handle.clearNow();
  expect(add.mock.calls.filter(([type]) => type === 'focus')).toHaveLength(2);
  add.mockRestore();
});

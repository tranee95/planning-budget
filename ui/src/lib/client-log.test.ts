import { afterEach, expect, it, vi } from 'vitest';
import type { ClientErrorDto } from '$lib/api/bindings';
import { ApiError } from '$lib/api/call';
import {
  describeError,
  installClientErrorLog,
  reportClientError,
  resetClientLog,
  routeOf
} from './client-log';

afterEach(() => {
  resetClientLog();
});

const sender = () => {
  const sent: ClientErrorDto[] = [];
  const send = (entry: ClientErrorDto): Promise<unknown> => {
    sent.push(entry);
    return Promise.resolve(null);
  };
  return { sent, send };
};

it('текст ошибки уходит только у встроенных типов, у остальных он пустой', () => {
  const builtIn = describeError(new TypeError('Failed to fetch dynamically imported module: x.js'));
  expect(builtIn.name).toBe('TypeError');
  expect(builtIn.message).toBe('Failed to fetch dynamically imported module: x.js');

  const custom = describeError(new Error('Категория «Продукты» не найдена, 12 500 ₽'));
  expect(custom).toMatchObject({ name: 'Error', message: '' });

  expect(describeError('строка с суммой 500 ₽')).toEqual({
    name: 'string',
    message: '',
    stack: ''
  });
  expect(describeError(null).name).toBe('object');
});

it('у ApiError в журнал идёт только код ошибки', () => {
  const error = new ApiError({ code: 'Validation', messageKey: 'errors.x', field: 'name' });
  expect(describeError(error)).toEqual({ name: 'ApiError', message: 'Validation', stack: '' });
});

it('маршрут без адреса приложения, параметров запроса и якоря', () => {
  expect(routeOf('http://localhost:1421/expenses?q=лента#x')).toBe('/expenses');
  expect(routeOf('tauri://localhost/analytics')).toBe('/analytics');
  expect(routeOf('/savings?month=2026-09')).toBe('/savings');
  expect(routeOf('http://localhost')).toBe('/');
});

it('одинаковая ошибка отправляется один раз, число разных ограничено', () => {
  const { sent, send } = sender();
  for (let i = 0; i < 5; i++) reportClientError('route', new TypeError('boom'), '/analytics', send);
  expect(sent).toHaveLength(1);
  expect(sent[0]).toMatchObject({ kind: 'route', name: 'TypeError', route: '/analytics' });

  for (let i = 0; i < 100; i++) {
    reportClientError('uncaught', new TypeError(`ошибка ${String(i)}`), '/', send);
  }
  expect(sent.length).toBeLessThanOrEqual(30);
});

it('сбой самой отправки ничего не ломает', async () => {
  const send = vi.fn(() => Promise.reject(new Error('нет моста IPC')));
  expect(() => {
    reportClientError('route', new TypeError('x'), '/a', send);
  }).not.toThrow();
  await Promise.resolve();
  expect(send).toHaveBeenCalledOnce();
});

it('необработанные исключения и отказы промисов окна попадают в журнал', () => {
  // События не диспатчатся в окно: vitest сам считает «error» сбоем теста.
  const handlers = new Map<string, EventListener>();
  const add = vi.spyOn(window, 'addEventListener').mockImplementation((type, handler) => {
    handlers.set(type, handler as EventListener);
  });
  const remove = vi.spyOn(window, 'removeEventListener').mockImplementation(() => undefined);
  const { sent, send } = sender();
  const off = installClientErrorLog(send);
  handlers.get('error')?.({ error: new RangeError('bad range'), message: '' } as unknown as Event);
  handlers.get('unhandledrejection')?.({ reason: new ReferenceError('nope') } as unknown as Event);
  off();
  expect(remove).toHaveBeenCalledTimes(2);
  expect(sent.map((e) => [e.kind, e.name])).toEqual([
    ['uncaught', 'RangeError'],
    ['rejection', 'ReferenceError']
  ]);
  add.mockRestore();
  remove.mockRestore();
});

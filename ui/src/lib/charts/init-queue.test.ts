import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { enqueueInit, pendingInits } from './init-queue';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

it('задачи идут по одной за такт, в порядке постановки', () => {
  const log: string[] = [];
  enqueueInit(() => log.push('a'));
  enqueueInit(() => log.push('b'));
  enqueueInit(() => log.push('c'));
  expect(log).toEqual([]);
  vi.advanceTimersToNextTimer();
  expect(log).toEqual(['a']);
  vi.advanceTimersToNextTimer();
  expect(log).toEqual(['a', 'b']);
  vi.runAllTimers();
  expect(log).toEqual(['a', 'b', 'c']);
  expect(pendingInits()).toBe(0);
});

it('отменённая задача не выполняется, остальные идут дальше', () => {
  const log: string[] = [];
  enqueueInit(() => log.push('a'));
  const cancel = enqueueInit(() => log.push('b'));
  enqueueInit(() => log.push('c'));
  cancel();
  vi.runAllTimers();
  expect(log).toEqual(['a', 'c']);
});

it('сбой одной задачи не останавливает очередь', () => {
  const log: string[] = [];
  const errors: unknown[] = [];
  enqueueInit(
    () => {
      throw new Error('boom');
    },
    (e) => errors.push(e)
  );
  enqueueInit(() => log.push('next'));
  vi.runAllTimers();
  expect(errors).toHaveLength(1);
  expect(log).toEqual(['next']);
});

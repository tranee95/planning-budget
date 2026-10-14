import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { trackScrolling } from './scroll-state';

let stop: () => void;

beforeEach(() => {
  vi.useFakeTimers();
  stop = trackScrolling(document, 100);
});

afterEach(() => {
  stop();
  vi.useRealTimers();
  delete document.documentElement.dataset.scrolling;
});

it('во время прокрутки на корне стоит data-scrolling, через паузу он снимается', () => {
  expect(document.documentElement.dataset.scrolling).toBeUndefined();
  window.dispatchEvent(new Event('scroll'));
  expect(document.documentElement.dataset.scrolling).toBe('1');
  vi.advanceTimersByTime(60);
  window.dispatchEvent(new Event('scroll'));
  vi.advanceTimersByTime(60);
  // Новая прокрутка продлила паузу: признак ещё стоит.
  expect(document.documentElement.dataset.scrolling).toBe('1');
  vi.advanceTimersByTime(60);
  expect(document.documentElement.dataset.scrolling).toBeUndefined();
});

it('после отписки признак не ставится', () => {
  stop();
  window.dispatchEvent(new Event('scroll'));
  expect(document.documentElement.dataset.scrolling).toBeUndefined();
});

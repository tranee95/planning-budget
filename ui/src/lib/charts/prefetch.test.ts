import { afterEach, expect, it, vi } from 'vitest';
import { prefetchCharts } from './prefetch';

afterEach(() => {
  vi.useRealTimers();
});

it('подгрузка ECharts откладывается до простоя и выполняется один раз', async () => {
  vi.useFakeTimers();
  const load = vi.fn(() => Promise.resolve({}));
  prefetchCharts(load);
  prefetchCharts(load);
  expect(load).not.toHaveBeenCalled();
  await vi.runAllTimersAsync();
  expect(load).toHaveBeenCalledTimes(1);
});

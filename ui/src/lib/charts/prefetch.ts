let started = false;

/**
 * Подгружает модуль графиков, когда приложение простаивает: первый график после входа не
 * ждёт загрузки ECharts. В основной бандл модуль не входит.
 * `load` передаётся для тестов.
 */
export function prefetchCharts(load: () => Promise<unknown> = () => import('./echarts')): void {
  if (started) return;
  started = true;
  const run = (): void => {
    void load().catch(() => {
      // Подгрузка необязательна: график загрузит модуль сам при первом показе.
      started = false;
    });
  };
  if ('requestIdleCallback' in globalThis) requestIdleCallback(run, { timeout: 3000 });
  else setTimeout(run, 1500);
}

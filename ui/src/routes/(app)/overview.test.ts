import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { planApi, summaryApi } from '$lib/api/data';
import { resetMockBudget, setMockToday } from '$lib/api/mock/budget';
import { resultCache } from '$lib/api/result-cache';
import { installMocks } from '$lib/api/mock/install';
import { formatMoney } from '$lib/format';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { OverviewVm, planLockedText } from './overview.svelte';

beforeEach(() => {
  setMockToday(new Date(2026, 8, 15));
  resultCache.clear();
  installMocks();
  resetMockBudget();
});

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  categories.reset();
});

it('load: сводка, год, настройки и лимиты', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.error).toBeNull();
  expect(vm.summary?.month).toBe('2026-09');
  expect(vm.settings?.savingsMinBp).toBe(1300);
  expect(vm.series).toHaveLength(12);
});

it('лимиты отсортированы по использованию, сбережения не входят', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  const usage = vm.limitLines.map((l) => l.row.usage ?? 0);
  expect(usage).toEqual([...usage].sort((a, b) => b - a));
  expect(vm.limitLines.every((l) => l.category.kind !== 'savings')).toBe(true);
});

it('подзаголовок: в мок-данных превышений нет', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.subtitle).toBe('Сентябрь: все категории в лимитах');
});

it('пустой месяц: нет дохода, сравнивать расходы не с чем', async () => {
  const vm = new OverviewVm();
  await vm.load('2025-01');
  expect(vm.savingsHint).toBe('нет дохода в месяце');
  expect(vm.overview?.expensesDeltaPercent).toBeNull();
});

it('«12 мес» берёт ровно двенадцать месяцев до выбранного', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.setRange('12m');
  expect(vm.series).toHaveLength(12);
  expect(vm.series.at(-1)?.month).toBe('2026-09');
});

it('«Всё» оставляет только месяцы с данными', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.setRange('all');
  // 2024 — история «Ленты» из мока (E2E 4); пустые 2025 и месяцы 2026 до сентября отброшены.
  expect(vm.series.map((m) => m.month)).toEqual([
    ...Array.from({ length: 8 }, (_, i) => `2024-0${String(i + 1)}`),
    '2026-09'
  ]);
});

it('смена периода графика во время загрузки не обрывает загрузку', async () => {
  const vm = new OverviewVm();
  const loading = vm.load('2026-09');
  const ranging = vm.setRange('12m');
  await Promise.all([loading, ranging]);
  expect(vm.loading).toBe(false);
  expect(vm.summary?.month).toBe('2026-09');
});

it('план месяца загружается вместе с обзором; строки без плана и факта не показываются', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.plan?.month).toBe('2026-09');
  expect(vm.plan?.locked).toBe(false);
  expect(vm.planLines.length).toBeGreaterThan(0);
  expect(vm.planLines.every((l) => l.row.plan !== 0 || l.row.fact !== 0)).toBe(true);
  // «Фактически» — часть «Расхода с планом»: плановые строки в неё не входят.
  expect(vm.planLines.every((l) => l.row.actual >= 0 && l.row.actual <= l.row.fact)).toBe(true);
  // В сентябре плановые траты есть: копировать план некуда.
  expect(vm.canCopyPlan).toBe(false);
});

it('«План готов» фиксирует месяц, «Разблокировать» снимает фиксацию', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.lockPlan();
  expect(vm.plan?.locked).toBe(true);
  expect(vm.overview?.planLocked).toBe(true);
  expect(vm.planBusy).toBe(false);
  await vm.unlockPlan();
  expect(vm.plan?.locked).toBe(false);
  expect(vm.overview?.planLocked).toBe(false);
});

it('текст тоста «План готов»: месяц и, если есть, строка о нераспределённом', () => {
  expect(planLockedText('2026-09', 0)).toBe('План на сентябрь 2026 зафиксирован');
  expect(planLockedText('2026-09', 1_234_500)).toBe(
    `План на сентябрь 2026 зафиксирован. Не распределено ${formatMoney(1_234_500)}`
  );
  expect(planLockedText('2026-03', -50_000)).toBe(
    `План на март 2026 зафиксирован. План превышает доход на ${formatMoney(50_000)}`
  );
});

it('«План готов» показывает тост с действием «Разблокировать», которое снимает фиксацию', async () => {
  toasts.clear();
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.lockPlan();
  const toast = toasts.items.find((t) =>
    t.message.startsWith('План на сентябрь 2026 зафиксирован')
  );
  expect(toast?.action?.label).toBe('Разблокировать');
  toast?.action?.run();
  await vi.waitFor(() => {
    expect(vm.plan?.locked).toBe(false);
  });
});

it('в пустом месяце предлагается скопировать план; повторное копирование не создаёт дублей', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-10');
  expect(vm.canCopyPlan).toBe(true);
  await vm.copyPlan();
  expect(vm.plan?.planExpenses).toBeGreaterThan(0);
  expect(vm.canCopyPlan).toBe(false);
  const planned = vm.plan?.planExpenses;
  await vm.copyPlan();
  expect(vm.plan?.planExpenses).toBe(planned);
});

it('ошибка действия с планом превращается в тост, а не обрывает экран', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.unlockPlan();
  expect(vm.error).toBeNull();
  expect(vm.planBusy).toBe(false);
});

it('погашение долга появляется в плане месяца и отмечается оплаченным', async () => {
  const { debtsApi } = await import('$lib/api/data');
  await debtsApi.create({
    lender: 'Кредитная карта',
    amount: 1_000_000,
    takenMonth: '2026-09',
    takenDate: null,
    categoryId: null,
    comment: null,
    schedule: [{ month: '2026-10', amount: 1_000_000 }]
  });
  const vm = new OverviewVm();
  await vm.load('2026-10');
  expect(vm.plan?.planRepayments).toBe(1_000_000);
  expect(vm.plan?.repayments).toHaveLength(1);
  const paymentId = vm.plan?.repayments[0]?.paymentId ?? 0;
  await vm.payRepayment(paymentId, true);
  expect(vm.plan?.repayments[0]?.status).toBe('paid');
  await vm.payRepayment(paymentId, false);
  expect(vm.plan?.repayments[0]?.status).toBe('planned');
});

// ---- Q-10: кеш расчётов «Обзора» ----

it('повторная загрузка без изменений данных берёт расчёты из кеша', async () => {
  const first = new OverviewVm();
  await first.load('2026-09');
  const month = vi.spyOn(summaryApi, 'month');
  const series = vi.spyOn(summaryApi, 'series');
  const plan = vi.spyOn(planApi, 'month');
  const again = new OverviewVm();
  await again.load('2026-09');
  expect(month).not.toHaveBeenCalled();
  expect(series).not.toHaveBeenCalled();
  expect(plan).not.toHaveBeenCalled();
  expect(again.summary?.month).toBe('2026-09');
  expect(again.series).toHaveLength(12);
});

it('другой месяц считается заново, а после изменения данных кеш очищен', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  const month = vi.spyOn(summaryApi, 'month');
  await vm.load('2026-08');
  expect(month).toHaveBeenCalledTimes(1);
  resultCache.clear();
  await vm.load('2026-08');
  expect(month).toHaveBeenCalledTimes(2);
});

it('график и остальные расчёты запрашиваются одновременно, а не по очереди', async () => {
  const order: string[] = [];
  const realMonth = summaryApi.month;
  const realSeries = summaryApi.series;
  vi.spyOn(summaryApi, 'month').mockImplementation(async (...a) => {
    order.push('month:start');
    const r = await realMonth(...a);
    order.push('month:end');
    return r;
  });
  vi.spyOn(summaryApi, 'series').mockImplementation(async (...a) => {
    order.push('series:start');
    const r = await realSeries(...a);
    order.push('series:end');
    return r;
  });
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(order.indexOf('series:start')).toBeLessThan(order.indexOf('month:end'));
});

it('на следующий день расчёты берутся заново: ключ кеша содержит дату', async () => {
  vi.useFakeTimers({ toFake: ['Date'] });
  try {
    vi.setSystemTime(new Date('2026-09-10T12:00:00'));
    const vm = new OverviewVm();
    await vm.load('2026-09');
    const month = vi.spyOn(summaryApi, 'month');
    await vm.load('2026-09');
    expect(month).not.toHaveBeenCalled();
    vi.setSystemTime(new Date('2026-09-11T12:00:00'));
    await vm.load('2026-09');
    expect(month).toHaveBeenCalledTimes(1);
  } finally {
    vi.useRealTimers();
  }
});

it('«На неделю»: подпись про оставшиеся недели только у текущего месяца', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.weeksHint).toBe('до конца месяца осталось 2,3 недели');
  await vm.load('2026-08');
  expect(vm.weeksHint).toBeNull();
});

it('«Свободный остаток по факту» — вторая строка под основным числом', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.freeActualHint).toBe(`по факту: ${formatMoney(vm.summary?.freeActual ?? 0)}`);
});

import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { installMocks } from '$lib/api/mock/install';
import { resetMockBudget } from '$lib/api/mock/budget';
import { toasts } from '$lib/stores/toasts.svelte';
import { BondsVm } from './bonds.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
});

afterEach(() => {
  clearMocks();
});

async function loaded(): Promise<BondsVm> {
  const vm = new BondsVm();
  vm.year = 2026;
  await vm.load();
  return vm;
}

it('load: факт за 12 месяцев, три сценария и черновик параметров из настроек', async () => {
  const vm = await loaded();
  expect(vm.data?.months).toHaveLength(12);
  expect(vm.data?.scenarios.map((s) => s.scenario)).toEqual(['a', 'b', 'c']);
  expect(vm.draft).toEqual({ rate: '16', tax: '13', balance: 0, month: '2026-01' });
  expect(vm.factChart?.series).toHaveLength(2);
  expect(vm.forecastChart?.categories).toHaveLength(60);
  expect(vm.hasContributions).toBe(true);
});

it('год ограничен стартовым месяцем накоплений и текущим годом', async () => {
  const vm = await loaded();
  await vm.setYear(2025);
  expect(vm.year).toBe(2026);
  await vm.setYear(2099);
  expect(vm.year).toBe(2026);
});

it('saveParams: неверный процент не уходит в настройки, верный сохраняется', async () => {
  const vm = await loaded();
  vm.draft.rate = 'abc';
  expect(await vm.saveParams()).toBe(false);
  expect(vm.paramsError).toContain('от 0 до 100');

  vm.draft.rate = '16,5';
  vm.draft.balance = 10_000_000;
  expect(await vm.saveParams()).toBe(true);
  expect(vm.paramsError).toBe('');
  expect(vm.settings?.bondsRateBp).toBe(1650);
  expect(vm.settings?.bondsInitialBalance).toBe(10_000_000);
  expect(toasts.items.at(-1)?.message).toBe('Параметры облигаций сохранены');
});

it('saveParams: пустой баланс или месяц — подсказка, запроса нет', async () => {
  const vm = await loaded();
  vm.draft.balance = null;
  expect(await vm.saveParams()).toBe(false);
  expect(vm.paramsError).toContain('стартовый баланс');
});

it('перезагрузка (смена года, событие данных) не стирает набираемое в форме', async () => {
  const vm = await loaded();
  vm.draft.rate = '17,5';
  await vm.load();
  expect(vm.draft.rate).toBe('17,5');
});

it('сохранение пересчитывает расчёт по новым параметрам', async () => {
  const vm = await loaded();
  const before = vm.data?.effectiveRate ?? 0;
  vm.draft.rate = '20';
  expect(await vm.saveParams()).toBe(true);
  expect(vm.data?.effectiveRate ?? 0).toBeGreaterThan(before);
  expect(vm.draft.rate).toBe('20');
});

it('стартовый месяц позже просматриваемого года: страница переходит на допустимый год', async () => {
  const vm = await loaded();
  vm.draft.month = '2026-03';
  expect(await vm.saveParams()).toBe(true);
  expect(vm.year).toBe(2026);
  vm.draft.month = '2099-01';
  expect(await vm.saveParams()).toBe(true);
  expect(vm.year).toBe(vm.maxYear);
});

it('устаревший ответ не затирает свежий', async () => {
  const vm = await loaded();
  const stale = vm.load();
  const fresh = vm.load();
  await Promise.all([stale, fresh]);
  expect(vm.loading).toBe(false);
  expect(vm.data?.year).toBe(2026);
});

import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { SavingsVm } from './savings.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.items = [];
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

it('загрузка: накопления, первое выбрано, форма заполнена из параметров', async () => {
  const vm = new SavingsVm();
  await vm.load();
  expect(vm.error).toBe('');
  expect(vm.data?.items).toHaveLength(1);
  expect(vm.selected?.categoryId).toBe(vm.data?.items[0]?.categoryId);
  expect(vm.nameOf(vm.selected?.categoryId ?? 0)).toBe('Накопления');
  expect(vm.draft.rate).toBe('16');
  expect(vm.draft.tax).toBe('13');
  expect(vm.draft.planKind).toBe('percent');
  expect(vm.draft.planPercent).toBe('14');
  expect(vm.selected?.effectiveRateBp).toBe(1392);
});

it('итоги: баланс, план месяца и сценарии приходят готовыми', async () => {
  const vm = new SavingsVm();
  await vm.load();
  expect(vm.data?.totalBalance).toBe(vm.selected?.balance);
  expect(vm.data?.totalScenarios.map((s) => s.scenario)).toEqual(['a', 'b', 'c']);
  expect(vm.selected?.scenarios[0]?.balances).toHaveLength(60);
  expect(vm.selected?.factChart.series[0]?.name).toBe('Баланс');
  expect(vm.selected?.forecastChart.categories[0]).toBe('Янв 2027');
});

it('переключатель года ограничен первым годом данных и текущим годом', async () => {
  const vm = new SavingsVm();
  await vm.load();
  const first = vm.minYear;
  const last = vm.maxYear;
  await vm.setYear(first - 1);
  expect(vm.year).toBe(last);
  await vm.setYear(last + 1);
  expect(vm.year).toBe(last);
  await vm.setYear(first);
  expect(vm.data?.year).toBe(first);
});

it('сохранение: ставка меняется, показывается тост, форма заполняется заново', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.draft.rate = '9,5';
  vm.draft.tax = '0';
  const saved = await vm.saveParams();
  expect(saved).toBe(true);
  expect(vm.paramsError).toBe('');
  expect(vm.selected?.params.annualRateBp).toBe(950);
  expect(vm.selected?.effectiveRateBp).toBe(950);
  expect(vm.draft.rate).toBe('9,5');
  expect(toasts.items.at(-1)?.message).toBe('Накопление сохранено');
});

it('ошибки ввода показываются у формы и ничего не сохраняют', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.draft.rate = 'шестнадцать';
  expect(await vm.saveParams()).toBe(false);
  expect(vm.paramsError).toContain('от 0 до 100');

  vm.draft.rate = '16';
  vm.draft.balance = null;
  expect(await vm.saveParams()).toBe(false);
  expect(vm.paramsError).toContain('стартовый баланс');

  vm.draft.balance = 0;
  vm.draft.planKind = 'fixed';
  vm.draft.planFixed = null;
  expect(await vm.saveParams()).toBe(false);
  expect(vm.paramsError).toContain('сумму плана');
  expect(vm.selected?.params.annualRateBp).toBe(1600);
});

it('план суммой: вид плана и сумма сохраняются, возврат к проценту сбрасывает сумму', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.draft.planKind = 'fixed';
  vm.draft.planFixed = 500_000;
  expect(await vm.saveParams()).toBe(true);
  expect(vm.selected?.planKind).toBe('fixed');
  expect(vm.selected?.planFixed).toBe(500_000);
  expect(vm.draft.planFixed).toBe(500_000);

  vm.draft.planKind = 'percent';
  vm.draft.planPercent = '10';
  expect(await vm.saveParams()).toBe(true);
  expect(vm.selected?.planKind).toBe('percent');
  expect(vm.selected?.planRateBp).toBe(1000);
});

it('ставка 0 — накопление без процентов', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.draft.rate = '0';
  expect(await vm.saveParams()).toBe(true);
  expect(vm.selected?.params.annualRateBp).toBe(0);
});

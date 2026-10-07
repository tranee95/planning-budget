import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { planApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { onboarding } from '$lib/stores/onboarding.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { WizardVm } from './wizard.svelte';

const MONTH = '2027-03';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.items = [];
  onboarding.closeWizard();
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

it('загрузка: статьи расходов и накопления из текущих данных, итог пуст', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  expect(vm.error).toBe('');
  expect(vm.lines.length).toBeGreaterThan(0);
  expect(vm.lines.every((l) => l.category.kind !== 'savings')).toBe(true);
  expect(vm.savings.map((s) => s.category.name)).toEqual(['Накопления']);
  expect(vm.savings[0]?.percent).toBe('14');
  expect(vm.preview?.balance).toBe('empty');
  expect(vm.monthName).toBe('Март 2027');
});

it('ввод собирается без пустых и нулевых строк', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes = [
    { name: 'Зарплата', amount: 10_000_000 },
    { name: '', amount: 500_000 },
    { name: 'Подработка', amount: null }
  ];
  const first = vm.lines[0];
  if (!first) throw new Error('нет статей');
  first.amount = 3_000_000;
  expect(vm.input.incomes).toEqual([{ sourceName: 'Зарплата', amount: 10_000_000 }]);
  expect(vm.input.lines).toEqual([{ categoryId: first.category.id, amount: 3_000_000 }]);
  expect(vm.input.savings[0]?.plan).toEqual({ kind: 'percent', rateBp: 1400 });
});

it('«Не распределено» пересчитывает бэкенд при изменении ввода', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  await vm.refreshPreview();
  expect(vm.preview?.income).toBe(10_000_000);
  expect(vm.preview?.planSavings).toBe(1_400_000);
  expect(vm.preview?.unallocated).toBe(8_600_000);
  expect(vm.preview?.balance).toBe('unallocated');

  const line = vm.lines[0];
  if (!line) throw new Error('нет статей');
  line.amount = 9_000_000;
  await vm.refreshPreview();
  expect(vm.preview?.balance).toBe('over');
  expect(vm.preview?.unallocated).toBeLessThan(0);
});

it('план суммой вместо процента', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  const saving = vm.savings[0];
  if (!saving) throw new Error('нет накопления');
  saving.kind = 'fixed';
  saving.fixed = 500_000;
  expect(vm.input.savings[0]?.plan).toEqual({ kind: 'fixed', amount: 500_000 });
  await vm.refreshPreview();
  expect(vm.preview?.planSavings).toBe(500_000);
});

it('шаги: вперёд, назад и последний шаг', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  expect(vm.step).toBe(0);
  vm.back();
  expect(vm.step).toBe(0);
  vm.next();
  vm.next();
  vm.next();
  expect(vm.last).toBe(true);
  vm.next();
  expect(vm.step).toBe(3);
  vm.back();
  expect(vm.step).toBe(2);
});

it('«План готов» записывает план, фиксирует месяц, показывает тост и обновляет экран', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  const line = vm.lines[0];
  if (!line) throw new Error('нет статей');
  line.amount = 3_000_000;
  onboarding.openWizard();
  const before = onboarding.refreshKey;
  await vm.apply();
  expect(vm.error).toBe('');
  expect(onboarding.wizardOpen).toBe(false);
  expect(onboarding.refreshKey).toBe(before + 1);
  expect(toasts.items.at(-1)?.message).toBe('План на Март 2027 готов');
  const plan = await planApi.month(MONTH);
  expect(plan.locked).toBe(true);
  expect(plan.income).toBe(10_000_000);
  expect(plan.planExpenses).toBe(3_000_000);
});

it('повторное применение к зафиксированному месяцу даёт ошибку и не закрывает мастер', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  await vm.apply();
  const again = new WizardVm(MONTH);
  await again.init();
  onboarding.openWizard();
  await again.apply();
  expect(again.error).toContain('зафиксирован');
  expect(onboarding.wizardOpen).toBe(true);
});

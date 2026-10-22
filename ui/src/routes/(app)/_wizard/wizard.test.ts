import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { planApi, transactionsApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { setMockFixedPlan } from '$lib/api/mock/savings';
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
  expect(vm.savings[0]?.fixed).toBeNull();
  expect(vm.preview?.balance).toBe('empty');
  expect(vm.monthName).toBe('Март 2027');
});

it('фиксированный план накопления приходит из плана месяца', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  const saving = vm.savings[0];
  if (!saving) throw new Error('нет категории накопления');
  setMockFixedPlan(saving.category.id, 700_000);
  const again = new WizardVm(MONTH);
  await again.init();
  expect(again.savings[0]?.fixed).toBe(700_000);
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
  expect(vm.input.savings[0]?.plan).toEqual({ kind: 'fixed', amount: 0 });
  const saving = vm.savings[0];
  if (!saving) throw new Error('нет накопления');
  saving.fixed = 700_000;
  expect(vm.input.savings[0]?.plan).toEqual({ kind: 'fixed', amount: 700_000 });
});

it('«Не распределено» пересчитывает бэкенд при изменении ввода', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  const saving = vm.savings[0];
  if (!saving) throw new Error('нет накопления');
  saving.fixed = 1_400_000;
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

it('сбережения: только сумма в месяц, процента в вводе нет', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  const saving = vm.savings[0];
  if (!saving) throw new Error('нет накопления');
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
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
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
  expect(toasts.items.at(-1)?.message).toBe('План на март 2027 готов');
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
  // Мастер теперь сам видит зафиксированный план при открытии и блокируется...
  expect(again.blockedReason).toContain('уже есть план');
  // ...но защита на стороне Rust остаётся: если проверка при открытии устарела, команда откажет.
  again.blockedReason = null;
  await again.apply();
  expect(again.error).toContain('зафиксирован');
  expect(onboarding.wizardOpen).toBe(true);
});

it('стор категорий загружен до засева и пуст: мастер перечитывает его и показывает статьи и накопления', async () => {
  // После знакомства стор помечен загруженным с пустым списком, а событие data-changed
  // перечитывает его позже, чем открывается мастер.
  categories.items = [];
  categories.loaded = true;
  const vm = new WizardVm(MONTH);
  await vm.init();
  expect(vm.error).toBe('');
  expect(vm.lines.length).toBeGreaterThan(0);
  expect(vm.savings.length).toBeGreaterThan(0);
});

it('месяц уже с планом: мастер сразу сообщает об этом и не даёт идти дальше (Q-16)', async () => {
  // В моке сентябрь 2026 с плановыми тратами (как после автопереноса U-1).
  const vm = new WizardVm('2026-09');
  await vm.init();
  expect(vm.blockedReason).toContain('уже есть план');
  vm.next();
  expect(vm.step).toBe(0);
  await vm.apply();
  expect(onboarding.wizardOpen).toBe(false);
});

it('пустой месяц: мастер не заблокирован', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  expect(vm.blockedReason).toBeNull();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  vm.next();
  expect(vm.step).toBe(1);
});

it('«Далее» на шаге «Доход» недоступна, пока нет дохода с суммой больше нуля', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  expect(vm.canNext).toBe(false);
  vm.next();
  expect(vm.step).toBe(0);
  vm.incomes[0] = { name: 'Зарплата', amount: 0 };
  expect(vm.canNext).toBe(false);
  vm.incomes[0] = { name: '', amount: 5_000_000 };
  expect(vm.canNext).toBe(false);
  vm.incomes[0] = { name: 'Зарплата', amount: 5_000_000 };
  expect(vm.canNext).toBe(true);
  vm.next();
  expect(vm.step).toBe(1);
  expect(vm.canNext).toBe(true);
});

async function overPlanned(): Promise<WizardVm> {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  const line = vm.lines[0];
  if (!line) throw new Error('нет статей');
  line.amount = 12_000_000;
  await vm.refreshPreview();
  vm.step = 3;
  return vm;
}

it('план превышает доход: первое «План готов» просит подтверждения и ничего не пишет', async () => {
  const vm = await overPlanned();
  await vm.apply();
  expect(vm.confirmingOver).toBe(true);
  expect(vm.overBy).toBe(2_000_000);
  expect((await planApi.month(MONTH)).locked).toBe(false);
});

it('«Назад к плану» снимает подтверждение; «Зафиксировать» записывает план', async () => {
  const vm = await overPlanned();
  await vm.apply();
  vm.cancelOver();
  expect(vm.confirmingOver).toBe(false);
  await vm.apply();
  expect(vm.confirmingOver).toBe(true);
  await vm.apply();
  expect(vm.error).toBe('');
  expect((await planApi.month(MONTH)).locked).toBe(true);
});

it('шаг назад снимает запрос подтверждения', async () => {
  const vm = await overPlanned();
  await vm.apply();
  vm.back();
  expect(vm.confirmingOver).toBe(false);
});

it('при «Не распределено» не меньше нуля «План готов» записывает сразу', async () => {
  const vm = new WizardVm(MONTH);
  await vm.init();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  await vm.refreshPreview();
  vm.step = 3;
  await vm.apply();
  expect(vm.confirmingOver).toBe(false);
  expect((await planApi.month(MONTH)).locked).toBe(true);
});

it('в заблокированном месяце подтверждение не запрашивается', async () => {
  const vm = await overPlanned();
  vm.blockedReason = 'занято';
  await vm.apply();
  expect(vm.confirmingOver).toBe(false);
});

it('в месяце только фактические траты без плановых строк мастер не блокируется', async () => {
  // Факты «Оплачено» без записей «План»: Rust такой месяц принимает, мастер тоже.
  await transactionsApi.create(crypto.randomUUID(), {
    month: MONTH,
    date: null,
    categoryId: 1,
    title: 'Магазин',
    amount: 150_000,
    status: 'paid',
    comment: null
  });
  const vm = new WizardVm(MONTH);
  await vm.init();
  expect(vm.blockedReason).toBeNull();
  vm.incomes[0] = { name: 'Зарплата', amount: 10_000_000 };
  vm.next();
  expect(vm.step).toBe(1);
});

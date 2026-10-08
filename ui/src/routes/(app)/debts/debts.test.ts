import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { DebtsVm } from './debts.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.items = [];
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

/** Новый долг на 30 000 ₽ равными частями на 3 месяца. */
async function createThreeMonthDebt(vm: DebtsVm): Promise<void> {
  vm.startNew();
  vm.draft.lender = 'Кредитная карта';
  vm.draft.amount = 3_000_000;
  vm.draft.months = '3';
  await vm.refreshSchedule();
  await vm.save();
}

it('пустой раздел: долгов нет, итоги нулевые', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  expect(vm.error).toBeNull();
  expect(vm.overview?.debts).toEqual([]);
  expect(vm.overview?.remainingTotal).toBe(0);
  expect(vm.subtitle).toBe('Открытых долгов нет');
});

it('быстрый график равными частями приходит от бэкенда и сходится с суммой', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  vm.startNew();
  vm.draft.amount = 1_000_001;
  vm.draft.months = '3';
  await vm.refreshSchedule();
  expect(vm.schedule.map((r) => r.month)).toEqual(['2026-10', '2026-11', '2026-12']);
  expect(vm.scheduleTotal).toBe(1_000_001);
  expect(vm.scheduleError).toBeNull();
});

it('график одним платежом; платёж раньше месяца займа даёт ошибку без сохранения', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  vm.startNew();
  vm.draft.amount = 500_000;
  vm.draft.kind = 'single';
  vm.draft.singleMonth = '2026-12';
  await vm.refreshSchedule();
  expect(vm.schedule).toEqual([{ month: '2026-12', amount: 500_000 }]);

  vm.draft.singleMonth = '2026-08';
  await vm.refreshSchedule();
  expect(vm.schedule).toEqual([]);
  expect(vm.scheduleError).not.toBeNull();
  vm.draft.lender = 'Брат';
  expect(vm.canSave).toBe(false);
});

it('создание долга: появляется в списке с остатком, формой и итогами', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  await createThreeMonthDebt(vm);
  expect(vm.selected).toBeNull();
  expect(vm.overview?.debts).toHaveLength(1);
  const debt = vm.overview?.debts[0];
  expect(debt?.remaining).toBe(3_000_000);
  expect(debt?.nextPayment?.month).toBe('2026-10');
  expect(vm.overview?.remainingTotal).toBe(3_000_000);
});

it('пустое имя не сохраняется', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  vm.startNew();
  vm.draft.amount = 3_000_000;
  await vm.refreshSchedule();
  expect(vm.canSave).toBe(false);
  vm.draft.lender = 'Карта';
  expect(vm.canSave).toBe(true);
});

it('оплата строк закрывает долг, закрытые скрыты по умолчанию и видны по переключателю', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-10');
  await createThreeMonthDebt(vm);
  const payments = vm.overview?.debts[0]?.payments ?? [];
  for (const p of payments) await vm.setPayment(p.id, true, '2026-10-05');
  expect(vm.overview?.debts).toEqual([]);
  expect(vm.overview?.closedCount).toBe(1);
  await vm.setIncludeClosed(true);
  expect(vm.overview?.debts[0]?.closed).toBe(true);
  expect(vm.overview?.debts[0]?.paidBp).toBe(10_000);
});

it('редактирование: график можно менять, пока нет оплаченных строк', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  await createThreeMonthDebt(vm);
  const id = vm.overview?.debts[0]?.id ?? 0;
  vm.select(id);
  expect(vm.scheduleEditable).toBe(true);

  const first = vm.overview?.debts[0]?.payments[0];
  await vm.setPayment(first?.id ?? 0, true, '2026-10-05');
  vm.select(id);
  expect(vm.scheduleEditable).toBe(false);
  vm.draft.lender = 'Рассрочка';
  await vm.save();
  expect(vm.overview?.debts[0]?.lender).toBe('Рассрочка');
  expect(vm.overview?.debts[0]?.remaining).toBe(2_000_000);
});

it('правка комментария не отправляет перестроенный график, неравный график сохраняется', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  vm.startNew();
  vm.draft.lender = 'Брат';
  vm.draft.amount = 3_000_000;
  vm.draft.kind = 'single';
  vm.draft.singleMonth = '2026-12';
  await vm.refreshSchedule();
  await vm.save();
  const debt = vm.overview?.debts[0];
  const id = debt?.id ?? 0;
  expect(debt?.payments.map((p) => p.month)).toEqual(['2026-12']);

  vm.select(id);
  await vm.refreshSchedule();
  expect(vm.schedule).toEqual([{ month: '2026-12', amount: 3_000_000 }]);
  vm.draft.comment = 'до зарплаты';
  await vm.refreshSchedule();
  await vm.save();
  const saved = vm.overview?.debts[0];
  expect(saved?.comment).toBe('до зарплаты');
  expect(saved?.payments.map((p) => p.month)).toEqual(['2026-12']);
});

it('правка месяцев пересобирает график, возврат к исходным значениям восстанавливает его', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  await createThreeMonthDebt(vm);
  vm.select(vm.overview?.debts[0]?.id ?? 0);
  vm.draft.months = '2';
  await vm.refreshSchedule();
  expect(vm.schedule).toHaveLength(2);
  vm.draft.months = '3';
  await vm.refreshSchedule();
  expect(vm.schedule.map((r) => r.month)).toEqual(['2026-10', '2026-11', '2026-12']);
});

it('удаление возвращается тостом «Вернуть»', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  await createThreeMonthDebt(vm);
  const debt = vm.overview?.debts[0];
  if (!debt) throw new Error('нет долга');
  await vm.remove(debt);
  expect(vm.overview?.debts).toEqual([]);
  const toast = toasts.items.at(-1);
  expect(toast?.action?.label).toBe('Вернуть');
  toast?.action?.run();
  await waitFor(() => vm.overview?.debts.length === 1);
  expect(vm.overview?.debts[0]?.lender).toBe('Кредитная карта');
});

it('долг из траты берёт сумму и статью из неё', async () => {
  const vm = new DebtsVm();
  await vm.load('2026-09');
  const { transactionsApi } = await import('$lib/api/data');
  const rows = await transactionsApi.list('2026-09');
  const tx = rows[0];
  if (!tx) throw new Error('в моке нет трат');
  await vm.startFromTransaction(tx.id, '2026-09');
  expect(vm.fromTransaction?.id).toBe(tx.id);
  expect(vm.draft.amount).toBe(tx.amount);
  expect(vm.draft.categoryId).toBe(tx.categoryId);
  vm.draft.lender = 'Рассрочка';
  vm.draft.months = '2';
  await vm.refreshSchedule();
  await vm.save();
  const debt = vm.overview?.debts[0];
  expect(debt?.transactionId).toBe(tx.id);
  expect(debt?.amount).toBe(tx.amount);
});

/** Ждёт асинхронное условие без таймеров (несколько тиков микрозадач). */
async function waitFor(cond: () => boolean): Promise<void> {
  for (let i = 0; i < 50 && !cond(); i++) await Promise.resolve();
  await new Promise((r) => setTimeout(r, 0));
}

import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { ExpensesVm } from './expenses.svelte';

// UX-6: обратная связь на смену статуса и правку строки.

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
  undoStack.clear();
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

async function loaded(): Promise<ExpensesVm> {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  return vm;
}

it('setStatus: тост «Статус: <новый>» с «Отменить» возвращает прежний статус', async () => {
  const vm = await loaded();
  const before = vm.transactions.find((t) => t.id === 1)?.status;
  await vm.setStatus(1, 'planned');
  const toast = toasts.items.find((t) => t.message === 'Статус: План');
  expect(toast?.action?.label).toBe('Отменить');
  toasts.runAction(toast?.id ?? 0);
  await vi.waitFor(() => {
    expect(vm.transactions.find((t) => t.id === 1)?.status).toBe(before);
  });
});

it('серия быстрых смен статуса оставляет один тост — последний', async () => {
  const vm = await loaded();
  const row = vm.transactions.find((t) => t.status === 'paid');
  if (!row) throw new Error('нет оплаченной строки');
  await vm.cycleStatus(row.id);
  await vm.cycleStatus(row.id);
  await vm.cycleStatus(row.id);
  const statusToasts = toasts.items.filter((t) => t.message.startsWith('Статус: '));
  expect(statusToasts.map((t) => t.message)).toEqual(['Статус: План']);
});

it('saveEdit: сумма меняется, тост «Сумма изменена» и ⌘Z возвращают сумму и итоги месяца', async () => {
  const vm = await loaded();
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строк');
  const spentBefore = vm.overview?.summary.expenses ?? 0;
  const remainingBefore = vm.overview?.limitsRemaining ?? 0;
  vm.startEdit(row.id);
  await vm.saveEdit({ title: row.title, amount: row.amount + 100_000 });
  expect(vm.transactions.find((t) => t.id === row.id)?.amount).toBe(row.amount + 100_000);
  expect(vm.overview?.summary.expenses).toBe(spentBefore + 100_000);
  const toast = toasts.items.find((t) => t.message === 'Сумма изменена');
  expect(toast?.action?.label).toBe('Отменить');
  expect(undoStack.canUndo).toBe(true);

  await undoStack.undo();
  expect(vm.transactions.find((t) => t.id === row.id)?.amount).toBe(row.amount);
  expect(vm.overview?.summary.expenses).toBe(spentBefore);
  expect(vm.overview?.limitsRemaining).toBe(remainingBefore);

  await undoStack.redo();
  expect(vm.transactions.find((t) => t.id === row.id)?.amount).toBe(row.amount + 100_000);
});

it('saveEdit: тост «Отменить» возвращает прежнюю сумму', async () => {
  const vm = await loaded();
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строк');
  vm.startEdit(row.id);
  await vm.saveEdit({ title: row.title, amount: row.amount + 5_000 });
  const toast = toasts.items.find((t) => t.message === 'Сумма изменена');
  toasts.runAction(toast?.id ?? 0);
  await vi.waitFor(() => {
    expect(vm.transactions.find((t) => t.id === row.id)?.amount).toBe(row.amount);
  });
});

it('saveEdit: изменение названия — тост «Строка изменена», отмена возвращает название', async () => {
  const vm = await loaded();
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строк');
  vm.startEdit(row.id);
  await vm.saveEdit({ title: 'Другое имя', amount: row.amount });
  expect(toasts.items.some((t) => t.message === 'Строка изменена')).toBe(true);
  await undoStack.undo();
  expect(vm.transactions.find((t) => t.id === row.id)?.title).toBe(row.title);
});

it('saveEdit без изменений: закрывает правку, не пишет в стек отмены и не показывает тост', async () => {
  const vm = await loaded();
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строк');
  vm.startEdit(row.id);
  await vm.saveEdit({ title: row.title, amount: row.amount });
  expect(vm.editingId).toBeNull();
  expect(undoStack.canUndo).toBe(false);
  expect(toasts.items).toHaveLength(0);
});

it('фокус правки: по умолчанию сумма, по названию — название', () => {
  const vm = new ExpensesVm();
  vm.startEdit(1);
  expect(vm.editFocus).toBe('amount');
  vm.startEdit(2, 'title');
  expect(vm.editFocus).toBe('title');
  vm.startEdit(3);
  expect(vm.editFocus).toBe('amount');
});

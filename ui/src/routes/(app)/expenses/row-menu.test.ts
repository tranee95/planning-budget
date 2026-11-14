import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { ExpensesVm } from './expenses.svelte';
import { rowMenuItems, statusOfItem } from './row-menu';

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

it('пункты меню: правка, теги, четыре статуса с отметкой текущего, выбор, удаление', async () => {
  const vm = await loaded();
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строки');
  const items = rowMenuItems(row, false, true);
  expect(items.map((i) => i.id)).toEqual([
    'edit',
    'tags',
    'status:paid',
    'status:debt',
    'status:unplanned',
    'status:planned',
    'check',
    'delete'
  ]);
  expect(items.filter((i) => i.checked === true).map((i) => i.id)).toEqual([
    `status:${row.status}`
  ]);
  expect(items.find((i) => i.id === 'delete')?.danger).toBe(true);
  expect(rowMenuItems(row, true, true).find((i) => i.id === 'check')?.label).toBe('Снять выбор');
});

it('statusOfItem: только пункты статуса', () => {
  expect(statusOfItem('status:debt')).toBe('debt');
  expect(statusOfItem('status:nope')).toBeNull();
  expect(statusOfItem('edit')).toBeNull();
});

it('меню: открытие, пункты строки, закрытие', async () => {
  const vm = await loaded();
  const id = vm.transactions[0]?.id ?? 0;
  expect(vm.menuItems).toEqual([]);
  vm.openMenu(id, 10, 20);
  expect(vm.menu).toEqual({ id, x: 10, y: 20 });
  expect(vm.menuItems.length).toBeGreaterThan(0);
  vm.closeMenu();
  expect(vm.menu).toBeNull();
});

it('runMenu: правка, теги, выбор, смена статуса', async () => {
  const vm = await loaded();
  const row = vm.transactions.find((t) => t.status !== 'debt');
  if (!row) throw new Error('нет строки');
  vm.openMenu(row.id, 0, 0);
  vm.runMenu('edit');
  expect(vm.editingId).toBe(row.id);
  vm.runMenu('tags');
  expect(vm.tagEditId).toBe(row.id);
  vm.runMenu('check');
  expect(vm.checked.has(row.id)).toBe(true);
  vm.runMenu('status:debt');
  await flush();
  expect(vm.transactions.find((t) => t.id === row.id)?.status).toBe('debt');
});

it('runMenu: удаление убирает строку, без открытого меню ничего не делает', async () => {
  const vm = await loaded();
  const id = vm.transactions[0]?.id ?? 0;
  vm.runMenu('delete');
  expect(vm.transactions.some((t) => t.id === id)).toBe(true);
  vm.openMenu(id, 0, 0);
  vm.runMenu('delete');
  await flush();
  expect(vm.transactions.some((t) => t.id === id)).toBe(false);
});

/** Дожидается завершённых промисов мок-IPC. */
async function flush(): Promise<void> {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
}

it('в блоках пункта «Выбрать» нет, в таблице есть', async () => {
  const vm = await loaded();
  const id = vm.transactions[0]?.id ?? 0;
  vm.openMenu(id, 0, 0);
  expect(vm.menuItems.some((i) => i.id === 'check')).toBe(false);
  expect(vm.menuItems.at(-1)?.separator).toBe(true);
  vm.mode = 'table';
  expect(vm.menuItems.some((i) => i.id === 'check')).toBe(true);
});

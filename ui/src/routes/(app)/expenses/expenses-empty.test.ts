import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { planApi, summaryApi, transactionsApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { ExpensesVm } from './expenses.svelte';

// UX-7: пустое состояние месяца без трат.

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
  undoStack.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  categories.reset();
});

it('пустой месяц без фильтра: пустое состояние, копирование плана доступно', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-10');
  expect(vm.showEmpty).toBe(true);
  expect(vm.planItems).toBe(0);
  expect(vm.emptyHint).toBeNull();
  expect(vm.canCopyPlan).toBe(true);
  expect(vm.noCategories).toBe(false);
});

it('в прошлом месяце нет данных: «Скопировать план» не предлагается', async () => {
  const vm = new ExpensesVm();
  await vm.load('2025-01');
  expect(vm.showEmpty).toBe(true);
  expect(vm.canCopyPlan).toBe(false);
});

it('месяц с тратами и сбой загрузки пустое состояние не показывают', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  expect(vm.showEmpty).toBe(false);
  vi.spyOn(summaryApi, 'month').mockRejectedValueOnce(new Error('boom'));
  await vm.load('2026-10');
  expect(vm.error).not.toBeNull();
  expect(vm.showEmpty).toBe(false);
});

it('пустой результат поиска в таблице — не пустое состояние месяца', async () => {
  const vm = new ExpensesVm();
  vm.mode = 'table';
  vm.filter.set('такого-нет-нигде');
  await vm.load('2026-09');
  expect(vm.filtered).toBe(true);
  expect(vm.showEmpty).toBe(false);
});

it('фильтр статуса, скрывший все строки, не превращает месяц в пустой', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleStatus('debt');
  vm.toggleStatus('planned');
  vm.toggleStatus('unplanned');
  expect(vm.visible.length).toBeLessThan(vm.spendingRows.length);
  expect(vm.showEmpty).toBe(false);
});

it('пустой месяц с планом: подсказка «В плане N статей», копировать нечего', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  await planApi.lock('2026-09');
  await Promise.all(vm.transactions.map((t) => transactionsApi.remove(t.id)));
  await vm.load('2026-09');
  expect(vm.showEmpty).toBe(true);
  expect(vm.planItems).toBeGreaterThan(0);
  expect(vm.emptyHint).toMatch(/^В плане \d+ стат/);
  expect(vm.canCopyPlan).toBe(false);
});

it('copyPlan: переносит план из прошлого месяца и перечитывает экран', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-10');
  await vm.copyPlan();
  expect(vm.showEmpty).toBe(false);
  expect(vm.transactions.length).toBeGreaterThan(0);
});

it('copyPlan: ошибка команды показывается тостом, экран остаётся пустым', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-10');
  vi.spyOn(planApi, 'copyFromPrevious').mockRejectedValueOnce(new Error('boom'));
  await vm.copyPlan();
  expect(toasts.items.at(-1)?.kind).toBe('error');
  expect(vm.showEmpty).toBe(true);
});

it('нет категорий расходов: подсказка про «Категории», копирование не предлагается', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-10');
  categories.items = [];
  expect(vm.noCategories).toBe(true);
  expect(vm.canCopyPlan).toBe(false);
});

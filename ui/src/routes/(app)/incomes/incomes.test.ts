import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { IncomesVm } from './incomes.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
  undoStack.clear();
});

afterEach(() => {
  clearMocks();
});

it('load: грузит доходы месяца', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  expect(vm.incomes.map((i) => i.sourceName)).toEqual(['Зарплата', 'Подработка']);
  expect(vm.error).toBeNull();
  expect(vm.isEmpty).toBe(false);
});

it('пустой месяц: isEmpty', async () => {
  const vm = new IncomesVm();
  await vm.load('2025-01');
  expect(vm.isEmpty).toBe(true);
});

it('фильтр по статусу: получено / ожидается', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  vm.statusFilter = 'expected';
  expect(vm.visible.map((i) => i.sourceName)).toEqual(['Подработка']);
  vm.statusFilter = null;
  expect(vm.visible).toHaveLength(2);
});

it('setStatus: оптимистично меняет статус и сохраняет его', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  const pending = vm.setStatus(2, 'received');
  expect(vm.incomes.find((i) => i.id === 2)?.status).toBe('received');
  await pending;
  await vm.load('2026-09');
  expect(vm.incomes.find((i) => i.id === 2)?.status).toBe('received');
});

it('setStatus: при ошибке откатывает строку и показывает тост', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  await vm.setStatus(999, 'received');
  expect(vm.incomes.find((i) => i.id === 2)?.status).toBe('expected');
  expect(toasts.items.at(-1)?.kind).toBe('error');
});

it('totals: получено, ожидается и всего', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  const { received, expected, total } = vm.totals;
  expect(received + expected).toBe(total);
  expect(total).toBe(vm.incomes.reduce((sum, i) => sum + i.amount, 0));
  expect(expected).toBe(
    vm.incomes.filter((i) => i.status === 'expected').reduce((s, i) => s + i.amount, 0)
  );
});

it('create: добавляет доход в месяц экрана и закрывает форму', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  vm.openEditor();
  await vm.create({ sourceName: ' Премия ', amount: 5_000_000, date: null, status: 'expected' });
  expect(vm.incomes.find((i) => i.sourceName === 'Премия')).toMatchObject({
    month: '2026-09',
    amount: 5_000_000,
    status: 'expected'
  });
  expect(vm.editorOpen).toBe(false);
});

it('create: пустой источник и сумма ≤ 0 не отправляются', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  await vm.create({ sourceName: ' ', amount: 100, date: null, status: 'received' });
  await vm.create({ sourceName: 'Премия', amount: 0, date: null, status: 'received' });
  expect(vm.incomes).toHaveLength(2);
});

it('undo/redo: создание и смена статуса', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  await vm.create({ sourceName: 'Премия', amount: 100, date: null, status: 'expected' });
  const id = vm.incomes.find((i) => i.sourceName === 'Премия')?.id ?? 0;
  await vm.setStatus(id, 'received');
  await undoStack.undo();
  expect(vm.incomes.find((i) => i.id === id)?.status).toBe('expected');
  await undoStack.undo();
  expect(vm.incomes.some((i) => i.id === id)).toBe(false);
  await undoStack.redo();
  expect(vm.incomes.some((i) => i.id === id)).toBe(true);
});

it('create: дата вне месяца экрана не отправляется', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  const ok = await vm.create({
    sourceName: 'Премия',
    amount: 100,
    date: '2026-10-15',
    status: 'received'
  });
  expect(ok).toBe(false);
  expect(vm.incomes).toHaveLength(2);
  expect(toasts.items.at(-1)?.kind).toBe('error');
});

it('totals: после смены статуса итоги берутся из новой сводки Rust', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  const before = vm.totals;
  await vm.setStatus(2, 'received');
  expect(vm.totals.total).toBe(before.total);
  expect(vm.totals.expected).toBe(0);
  expect(vm.totals.received).toBe(before.total);
});

it('фильтр: запрос ищет доходы по всем месяцам, пустой возвращает месяц', async () => {
  const vm = new IncomesVm();
  vm.filter.set('зарплата сумма>50000');
  await vm.load('2026-08');
  expect(vm.filtered).toBe(true);
  expect(vm.incomes.map((i) => i.sourceName)).toEqual(['Зарплата']);
  expect(vm.incomes[0]?.month).toBe('2026-09');
  expect(vm.filter.meta).toMatchObject({ total: 1, sum: 9_000_000 });

  vm.filter.clear();
  await vm.load('2026-09');
  expect(vm.incomes).toHaveLength(2);
  expect(vm.filter.meta).toBeNull();
});

it('статус трат исключает доходы из результата', async () => {
  const vm = new IncomesVm();
  vm.filter.set('статус:оплачено');
  await vm.load('2026-09');
  expect(vm.incomes).toEqual([]);
});

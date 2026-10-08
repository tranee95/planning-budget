import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { transactionsApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { ExpensesVm } from './expenses.svelte';
import { QuickAddVm } from './quick-add.svelte';

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

async function open(today = '2026-09-30'): Promise<{ expenses: ExpensesVm; vm: QuickAddVm }> {
  const expenses = new ExpensesVm();
  await expenses.load('2026-09');
  return { expenses, vm: new QuickAddVm(expenses, () => today) };
}

it('начальное состояние: статус «Оплачено», дата — сегодня, если она в месяце экрана', async () => {
  const { vm } = await open('2026-09-30');
  expect(vm.status).toBe('paid');
  expect(vm.date).toBe('2026-09-30');
  expect(vm.amount).toBeNull();
  expect(vm.canSave).toBe(false);
});

it('сегодня вне месяца экрана — дата пустая', async () => {
  const { vm } = await open('2026-10-02');
  expect(vm.date).toBe('');
});

it('chips: категории по частоте использования в истории, при равенстве — по порядку', async () => {
  const { vm } = await open();
  await vi.waitFor(() => {
    expect(vm.usage.size).toBeGreaterThan(0);
  });
  expect(vm.chips.map((c) => c.name)).toEqual(['Продукты', 'Жильё', 'Развлечения', 'Накопления']);
});

it('canSave требует сумму, категорию и наименование', async () => {
  const { vm } = await open();
  vm.amount = 259_600;
  expect(vm.canSave).toBe(false);
  vm.categoryId = 1;
  expect(vm.canSave).toBe(false);
  vm.title = '  Шары ';
  expect(vm.canSave).toBe(true);
  vm.amount = 0;
  expect(vm.canSave).toBe(false);
});

it('limitWarning: показывается, только если трата превышает остаток лимита', async () => {
  const { vm } = await open();
  vm.categoryId = 1;
  vm.amount = 1_000_000;
  expect(vm.limitWarning).toBeNull();
  vm.amount = 1_300_000;
  expect(vm.limitWarning).toEqual({ category: 'Продукты', over: 30_000 });
});

it('limitWarning: у категории без лимита предупреждения нет', async () => {
  const { vm } = await open();
  vm.categoryId = 4;
  vm.amount = 99_000_000;
  expect(vm.limitWarning).toBeNull();
});

it('suggestions: наименования из всей истории по вводу, без повторов, регистр не важен', async () => {
  const { vm } = await open();
  vm.title = 'ма';
  await vi.waitFor(() => {
    expect(vm.suggestions.map((s) => s.title)).toEqual(['Магазин у дома']);
  });
  vm.title = 'лен';
  await vi.waitFor(() => {
    expect(vm.suggestions.map((s) => [s.title, s.uses])).toEqual([['Лента', 8]]);
  });
  vm.title = 'ЛЕНТА';
  await vi.waitFor(() => {
    expect(vm.suggestions).toEqual([]);
  });
  vm.title = '';
  expect(vm.suggestions).toEqual([]);
});

it('suggestions: устаревший ответ не затирает свежий ввод', async () => {
  const { vm } = await open();
  vm.title = 'ры';
  vm.title = '';
  await new Promise((resolve) => setTimeout(resolve, 150));
  expect(vm.suggestions).toEqual([]);
});

it('pickSuggestion: подставляет наименование и категорию из истории', async () => {
  const { vm } = await open();
  vm.title = 'ры';
  await vi.waitFor(() => {
    expect(vm.suggestions).toHaveLength(1);
  });
  const [first] = vm.suggestions;
  expect(first?.title).toBe('Рынок');
  if (first) vm.pickSuggestion(first);
  expect(vm.title).toBe('Рынок');
  expect(vm.categoryId).toBe(1);
  expect(vm.suggestions).toEqual([]);
});

it('save: создаёт трату в месяце экрана, закрывает форму и перечитывает экран', async () => {
  const { expenses, vm } = await open();
  expenses.openEditor();
  vm.amount = 259_600;
  vm.categoryId = 1;
  vm.title = ' Шары ';
  await vm.save(false);
  const created = expenses.transactions.find((t) => t.title === 'Шары');
  expect(created).toMatchObject({
    amount: 259_600,
    categoryId: 1,
    status: 'paid',
    month: '2026-09',
    date: '2026-09-30'
  });
  expect(expenses.editorOpen).toBe(false);
  expect(toasts.items.at(-1)?.kind).toBe('info');
});

it('save(true): «ещё одну» оставляет форму открытой, очищает сумму и наименование', async () => {
  const { expenses, vm } = await open();
  expenses.openEditor();
  vm.amount = 10_000;
  vm.categoryId = 2;
  vm.title = 'Свет';
  vm.status = 'planned';
  await vm.save(true);
  expect(expenses.editorOpen).toBe(true);
  expect(vm.amount).toBeNull();
  expect(vm.title).toBe('');
  expect(vm.categoryId).toBe(2);
  expect(vm.status).toBe('planned');
  expect(vm.canSave).toBe(false);
});

it('save без обязательных полей ничего не создаёт', async () => {
  const { expenses, vm } = await open();
  const before = expenses.transactions.length;
  await vm.save(false);
  expect(expenses.transactions).toHaveLength(before);
});

it('save: создание можно отменить через стек действий', async () => {
  const { expenses, vm } = await open();
  vm.amount = 100;
  vm.categoryId = 1;
  vm.title = 'Шары';
  await vm.save(true);
  await undoStack.undo();
  expect(expenses.transactions.some((t) => t.title === 'Шары')).toBe(false);
});

it('dateError: дата вне месяца экрана блокирует сохранение', async () => {
  const { vm } = await open();
  vm.amount = 100;
  vm.categoryId = 1;
  vm.title = 'Шары';
  vm.date = '2026-10-15';
  expect(vm.dateError).not.toBeNull();
  expect(vm.canSave).toBe(false);
  vm.date = '2026-09-30';
  expect(vm.dateError).toBeNull();
  expect(vm.canSave).toBe(true);
});

it('save: каждая новая трата уходит со своим request_id, повтор того же id не создаёт дубль', async () => {
  const create = vi.spyOn(transactionsApi, 'create');
  const { expenses, vm } = await open();
  expenses.openEditor();
  vm.categoryId = 1;
  for (const title of ['Раз', 'Два']) {
    vm.amount = 10_000;
    vm.title = title;
    await vm.save(true);
  }
  const [first, second] = create.mock.calls.map(([requestId]) => requestId);
  expect(first).toBeTruthy();
  expect(second).not.toBe(first);

  const input = create.mock.calls[0]?.[1];
  if (!input) throw new Error('create не вызывался');
  const before = expenses.transactions.length;
  const again = await transactionsApi.create(first ?? '', input);
  expect(again.title).toBe('Раз');
  await expenses.load('2026-09');
  expect(expenses.transactions).toHaveLength(before);
  create.mockRestore();
});

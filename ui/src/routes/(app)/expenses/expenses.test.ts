import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { summaryApi, transactionsApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { ExpensesVm } from './expenses.svelte';

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

it('load: грузит траты, сводку и категории месяца', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  expect(vm.loading).toBe(false);
  expect(vm.error).toBeNull();
  expect(vm.transactions).toHaveLength(6);
  expect(vm.overview?.summary.month).toBe('2026-09');
  expect(vm.isEmpty).toBe(false);
});

it('пустой месяц: isEmpty, блоки категорий остаются', async () => {
  const vm = new ExpensesVm();
  await vm.load('2025-01');
  expect(vm.isEmpty).toBe(true);
  expect(vm.blocks).toHaveLength(4);
  expect(vm.blocks.every((b) => b.rows.length === 0)).toBe(true);
});

it('blocks: строки сгруппированы по категориям, лимит берётся из сводки', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const groceries = vm.blocks.find((b) => b.category.name === 'Продукты');
  expect(groceries?.rows.map((r) => r.title)).toEqual(['Магазин у дома', 'Рынок']);
  expect(groceries?.limit?.limit).toBe(3_000_000);
});

it('фильтр по статусу оставляет только выбранные', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleStatus('debt');
  expect(vm.visible.map((t) => t.title)).toEqual(['Кино']);
  expect(vm.blocks.flatMap((b) => b.rows)).toHaveLength(1);
  vm.toggleStatus('debt');
  expect(vm.visible).toHaveLength(6);
});

it('setStatus: оптимистично меняет строку и перечитывает сводку', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const before = vm.overview?.summary.byStatus.paid ?? 0;
  const pending = vm.setStatus(4, 'paid');
  expect(vm.transactions.find((t) => t.id === 4)?.status).toBe('paid');
  await pending;
  expect(vm.overview?.summary.byStatus.paid).toBe(before + 90_000);
});

it('setStatus: при ошибке откатывает строку и показывает тост', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  await vm.setStatus(999, 'paid');
  expect(vm.transactions.find((t) => t.id === 4)?.status).toBe('planned');
  expect(toasts.items.at(-1)?.kind).toBe('error');
});

it('устаревший ответ при быстрой смене месяца не затирает свежий', async () => {
  const vm = new ExpensesVm();
  const first = vm.load('2026-09');
  const second = vm.load('2025-01');
  await Promise.all([first, second]);
  expect(vm.transactions).toHaveLength(0);
});

it('режим и выбор строки — локальное состояние экрана', () => {
  const vm = new ExpensesVm();
  expect(vm.mode).toBe('blocks');
  vm.mode = 'table';
  vm.select(3);
  expect(vm.selected).toBe(3);
  vm.select(null);
  expect(vm.selected).toBeNull();
});

it('cycleStatus: идёт по кругу оплачено → долг → незапланировано → план', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const row = vm.transactions.find((t) => t.status === 'paid');
  expect(row).toBeDefined();
  if (!row) return;
  await vm.cycleStatus(row.id);
  expect(vm.transactions.find((t) => t.id === row.id)?.status).toBe('debt');
  await vm.cycleStatus(row.id);
  await vm.cycleStatus(row.id);
  await vm.cycleStatus(row.id);
  expect(vm.transactions.find((t) => t.id === row.id)?.status).toBe('paid');
});

it('правка строки: startEdit → saveEdit обновляет поля и перечитывает сводку', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строк');
  vm.startEdit(row.id);
  expect(vm.editingId).toBe(row.id);
  await vm.saveEdit({ title: 'Новое имя', amount: 12_345 });
  expect(vm.editingId).toBeNull();
  const saved = vm.transactions.find((t) => t.id === row.id);
  expect(saved?.title).toBe('Новое имя');
  expect(saved?.amount).toBe(12_345);
});

it('saveEdit: пустое наименование не отправляется, режим правки остаётся', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const row = vm.transactions[0];
  if (!row) throw new Error('нет строк');
  vm.startEdit(row.id);
  await vm.saveEdit({ title: '   ' });
  expect(vm.editingId).toBe(row.id);
  expect(vm.transactions[0]?.title).toBe(row.title);
});

it('cancelEdit сбрасывает правку', () => {
  const vm = new ExpensesVm();
  vm.startEdit(2);
  vm.cancelEdit();
  expect(vm.editingId).toBeNull();
});

it('добавление в карточку: startAdd → commitAdd создаёт строку плана в месяце экрана', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const category = vm.blocks[0]?.category;
  if (!category) throw new Error('нет категорий');
  const before = vm.transactions.length;
  vm.startAdd(category.id);
  expect(vm.addingTo).toBe(category.id);
  await vm.commitAdd({ title: 'Хлеб', amount: 9_000 });
  expect(vm.addingTo).toBeNull();
  expect(vm.transactions).toHaveLength(before + 1);
  const added = vm.transactions.at(-1);
  expect(added).toMatchObject({
    title: 'Хлеб',
    amount: 9_000,
    categoryId: category.id,
    month: '2026-09',
    status: 'planned'
  });
});

it('commitAdd: нулевая сумма или пустое имя не создают строку', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const category = vm.blocks[0]?.category;
  if (!category) throw new Error('нет категорий');
  const before = vm.transactions.length;
  vm.startAdd(category.id);
  await vm.commitAdd({ title: 'Хлеб', amount: 0 });
  await vm.commitAdd({ title: '', amount: 100 });
  expect(vm.transactions).toHaveLength(before);
  expect(vm.addingTo).toBe(category.id);
});

it('totals: расходы, лимиты и остаток берутся из сводки Rust', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  expect(vm.totals).toEqual({
    spent: vm.overview?.summary.expenses,
    limits: vm.overview?.limitsTotal,
    remaining: vm.overview?.limitsRemaining
  });
});

it('sortedRows: по умолчанию в порядке ввода, сортировка по сумме и повторный клик меняют направление', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  expect(vm.sortedRows.map((t) => t.id)).toEqual([1, 2, 3, 4, 5, 6]);
  vm.toggleSort('amount');
  expect(vm.sort).toEqual({ column: 'amount', descending: false });
  expect(vm.sortedRows.map((t) => t.amount)).toEqual([
    90_000, 140_000, 480_000, 1_250_000, 2_000_000, 4_200_000
  ]);
  vm.toggleSort('amount');
  expect(vm.sortedRows[0]?.amount).toBe(4_200_000);
  vm.toggleSort('amount');
  expect(vm.sort).toBeNull();
});

it('sortedRows: по категории сортирует по названию, по наименованию — по алфавиту', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleSort('title');
  expect(vm.sortedRows.map((t) => t.title)).toEqual(
    [...vm.transactions.map((t) => t.title)].sort((a, b) => a.localeCompare(b, 'ru'))
  );
  vm.toggleSort('category');
  const names = vm.sortedRows.map(
    (t) => categories.items.find((c) => c.id === t.categoryId)?.name ?? ''
  );
  expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b, 'ru')));
});

it('sortedRows учитывает фильтр статусов', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleStatus('planned');
  expect(vm.sortedRows.map((t) => t.id)).toEqual([4, 6]);
});

it('мультивыбор: toggleChecked, toggleAllChecked, clearChecked', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(2);
  vm.toggleChecked(3);
  expect([...vm.checked].sort()).toEqual([2, 3]);
  vm.toggleChecked(2);
  expect([...vm.checked]).toEqual([3]);
  vm.toggleAllChecked();
  expect(vm.checked.size).toBe(6);
  vm.toggleAllChecked();
  expect(vm.checked.size).toBe(0);
  vm.toggleChecked(1);
  vm.clearChecked();
  expect(vm.checked.size).toBe(0);
});

it('фильтр статусов выбрасывает из выбора скрытые строки', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleAllChecked();
  vm.toggleStatus('debt');
  expect([...vm.checkedVisible]).toEqual([5]);
});

it('bulkSetStatus: меняет статус выбранных, сбрасывает выбор и перечитывает сводку', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(4);
  vm.toggleChecked(6);
  await vm.bulkSetStatus('paid');
  expect(vm.transactions.filter((t) => t.status === 'paid').map((t) => t.id)).toEqual([1, 3, 4, 6]);
  expect(vm.checked.size).toBe(0);
  expect(vm.overview?.summary.byStatus.planned).toBe(0);
});

it('bulkSetCategory: переносит выбранные в другую категорию', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(1);
  vm.toggleChecked(2);
  await vm.bulkSetCategory(3);
  expect(vm.transactions.filter((t) => t.categoryId === 3).map((t) => t.id)).toEqual([1, 2, 5]);
  expect(vm.checked.size).toBe(0);
});

it('bulk: ошибка одной строки не теряет остальные и сообщается тостом', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(4);
  vm.toggleChecked(6);
  await transactionsApi.remove(6);
  await vm.bulkSetStatus('paid');
  expect(vm.transactions.find((t) => t.id === 4)?.status).toBe('paid');
  expect(toasts.items.at(-1)?.kind).toBe('error');
});

it('смена месяца сбрасывает выбор', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(1);
  await vm.load('2026-08');
  expect(vm.checked.size).toBe(0);
});

it('undo/redo: смена статуса', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const before = vm.transactions.find((t) => t.id === 1)?.status;
  await vm.setStatus(1, before === 'paid' ? 'planned' : 'paid');
  await undoStack.undo();
  expect(vm.transactions.find((t) => t.id === 1)?.status).toBe(before);
  await undoStack.redo();
  expect(vm.transactions.find((t) => t.id === 1)?.status).not.toBe(before);
});

it('undo/redo: создание строки', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.startAdd(1);
  await vm.commitAdd({ title: 'Кофе', amount: 200 });
  const created = vm.transactions.find((t) => t.title === 'Кофе');
  expect(created).toBeDefined();
  await undoStack.undo();
  expect(vm.transactions.some((t) => t.title === 'Кофе')).toBe(false);
  await undoStack.redo();
  expect(vm.transactions.some((t) => t.id === created?.id)).toBe(true);
});

it('bulkDelete: удаляет выбранные, undo возвращает', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(1);
  vm.toggleChecked(2);
  await vm.bulkDelete();
  expect(vm.transactions.some((t) => t.id === 1 || t.id === 2)).toBe(false);
  expect(vm.checked.size).toBe(0);
  await undoStack.undo();
  expect(vm.transactions.filter((t) => t.id === 1 || t.id === 2)).toHaveLength(2);
});

it('bulkDelete: тост «Отменить» возвращает строки', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(3);
  await vm.bulkDelete();
  const toast = toasts.items.at(-1);
  expect(toast?.action?.label).toBe('Отменить');
  toasts.runAction(toast?.id ?? 0);
  await vi.waitFor(() => {
    expect(vm.transactions.some((t) => t.id === 3)).toBe(true);
  });
});

it('тост «Отменить» после удаления отменяет удаление, даже если потом было другое действие', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.toggleChecked(3);
  await vm.bulkDelete();
  const toast = toasts.items.at(-1);
  const before = vm.transactions.find((t) => t.id === 1)?.status;
  await vm.setStatus(1, before === 'paid' ? 'planned' : 'paid');
  toasts.runAction(toast?.id ?? 0);
  await vi.waitFor(() => {
    expect(vm.transactions.some((t) => t.id === 3)).toBe(true);
  });
  expect(vm.transactions.find((t) => t.id === 1)?.status).not.toBe(before);
});

it('setStatus: сбой чтения итогов не откатывает сохранённый статус и не теряет undo', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  const before = vm.transactions.find((t) => t.id === 1)?.status;
  const next = before === 'paid' ? 'planned' : 'paid';
  vi.spyOn(summaryApi, 'month').mockRejectedValueOnce(new Error('boom'));
  await vm.setStatus(1, next);
  expect(vm.transactions.find((t) => t.id === 1)?.status).toBe(next);
  expect(toasts.items.at(-1)?.kind).toBe('error');
  expect(undoStack.canUndo).toBe(true);
  vi.restoreAllMocks();
});

it('фильтр таблицы: «лента статус:оплачено» даёт 7 трат на 90 681 ₽ из всех месяцев (E2E 4)', async () => {
  const vm = new ExpensesVm();
  vm.mode = 'table';
  vm.filter.set('лента статус:оплачено');
  await vm.load('2026-09');
  expect(vm.filtered).toBe(true);
  expect(vm.transactions).toHaveLength(7);
  expect(vm.transactions.every((t) => t.title === 'Лента' && t.status === 'paid')).toBe(true);
  expect(vm.filter.meta).toMatchObject({ total: 7, sum: 9_068_100, truncated: false });
  expect(vm.filter.meta?.spans.map((s) => s.kind)).toEqual(['text', 'status']);

  vm.filter.clear();
  await vm.load('2026-09');
  expect(vm.filtered).toBe(false);
  expect(vm.transactions).toHaveLength(6);
  expect(vm.filter.meta).toBeNull();
});

it('запрос не действует в режиме «Блоки»: показан месяц', async () => {
  const vm = new ExpensesVm();
  vm.filter.set('лента');
  await vm.load('2026-09');
  expect(vm.transactions).toHaveLength(6);
  expect(vm.filtered).toBe(false);
});

it('теги: setTags сохраняет выбор и перечитывает строки', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.tagEditId = 1;
  await vm.setTags(1, [1, 2]);
  expect(vm.tagEditId).toBeNull();
  expect(vm.transactions.find((t) => t.id === 1)?.tagIds).toEqual([1, 2]);
});

it('поиск по тегу: «#подарки» находит траты с этим тегом', async () => {
  const vm = new ExpensesVm();
  vm.mode = 'table';
  vm.filter.set('#подарки');
  await vm.load('2026-09');
  expect(vm.transactions.map((t) => t.id)).toEqual([18]);
});

it('правка по ссылке: строка открывается после загрузки, а скрытая фильтром — не «стреляет» позже', async () => {
  const vm = new ExpensesVm();
  await vm.load('2026-09');
  vm.openForEdit(1);
  expect(vm.editingId).toBe(1);
  vm.cancelEdit();

  vm.mode = 'table';
  vm.filter.set('статус:план');
  await vm.load('2026-09');
  vm.openForEdit(1);
  await vm.load('2026-09');
  expect(vm.editingId).toBeNull();

  vm.filter.clear();
  await vm.load('2026-09');
  expect(vm.editingId).toBeNull();
});

it('в режиме поиска правка статуса обновляет итог «Найдено»', async () => {
  const vm = new ExpensesVm();
  vm.mode = 'table';
  vm.filter.set('лента статус:оплачено');
  await vm.load('2026-09');
  expect(vm.filter.meta?.total).toBe(7);
  await vm.setStatus(11, 'planned');
  expect(vm.filter.meta).toMatchObject({ total: 6, sum: 9_068_100 - 980_000 });
});

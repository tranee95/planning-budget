import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { categoriesApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { DEFAULT_AMOUNT_TITLE, SavingsVm } from './savings.svelte';

const MONTH = '2027-03';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.items = [];
  undoStack.clear();
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

async function opened(month = MONTH): Promise<SavingsVm> {
  const vm = new SavingsVm();
  await vm.load(month);
  return vm;
}

async function put(vm: SavingsVm, categoryId: number, amount: number, title = ''): Promise<void> {
  vm.startAdd(categoryId);
  vm.draftTitle = title;
  vm.draftAmount = amount;
  expect(await vm.commitAdd()).toBe(true);
}

it('загрузка: одна карточка накопления, суммы месяца пусты, каждый месяц начинается пустым', async () => {
  const vm = await opened();
  expect(vm.error).toBe('');
  expect(vm.loading).toBe(false);
  expect(vm.cards).toHaveLength(1);
  expect(vm.cards[0]?.name).toBe('Накопления');
  expect(vm.cards[0]?.amounts).toEqual([]);
  expect(vm.monthTotal).toBe(0);
});

it('30 000 ₽ тремя разами по 10 000 ₽: три суммы одной карточки, а не три карточки', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await put(vm, id, 1_000_000, 'Аванс');
  await put(vm, id, 1_000_000);
  await put(vm, id, 1_000_000);
  expect(vm.cards).toHaveLength(1);
  const card = vm.cards[0];
  expect(card?.amounts).toHaveLength(3);
  expect(card?.total).toBe(3_000_000);
  expect(vm.monthTotal).toBe(3_000_000);
  expect(card?.amounts.map((t) => t.title)).toEqual([
    'Аванс',
    DEFAULT_AMOUNT_TITLE,
    DEFAULT_AMOUNT_TITLE
  ]);
  expect(card?.amounts.every((t) => t.status === 'paid')).toBe(true);
});

it('суммы принадлежат своему месяцу: в другом месяце карточка пуста', async () => {
  const vm = await opened();
  await put(vm, vm.cards[0]?.categoryId ?? 0, 500_000);
  await vm.load('2027-04');
  expect(vm.cards[0]?.amounts).toEqual([]);
  expect(vm.monthTotal).toBe(0);
  await vm.load(MONTH);
  expect(vm.monthTotal).toBe(500_000);
});

it('commitAdd(true): строка ввода остаётся открытой, черновик очищен, суммы копятся', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  vm.startAdd(id);
  for (let i = 0; i < 3; i++) {
    vm.draftAmount = 1_000_000;
    expect(await vm.commitAdd(true)).toBe(true);
    expect(vm.addingTo).toBe(id);
    expect(vm.draftAmount).toBeNull();
    expect(vm.draftTitle).toBe('');
  }
  expect(vm.cards[0]?.amounts).toHaveLength(3);
  expect(vm.cards[0]?.total).toBe(3_000_000);
});

it('commitAdd() без «ещё одной» закрывает строку ввода, как раньше', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  vm.startAdd(id);
  vm.draftAmount = 500_000;
  expect(await vm.commitAdd()).toBe(true);
  expect(vm.addingTo).toBeNull();
});

it('пустая или нулевая сумма не добавляется', async () => {
  const vm = await opened();
  vm.startAdd(vm.cards[0]?.categoryId ?? 0);
  expect(vm.canCommitAdd).toBe(false);
  vm.draftAmount = 0;
  expect(vm.canCommitAdd).toBe(false);
  expect(await vm.commitAdd()).toBe(false);
  vm.draftAmount = 100_000;
  expect(vm.canCommitAdd).toBe(true);
  vm.cancelAdd();
  expect(vm.addingTo).toBeNull();
  expect(vm.draftAmount).toBeNull();
});

it('сумму можно изменить: название и сумма, итог пересчитывается; ⌘Z возвращает прежнее', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await put(vm, id, 1_000_000, 'Аванс');
  await put(vm, id, 1_000_000, 'Премия');
  const line = vm.cards[0]?.amounts[0];
  if (!line) throw new Error('нет суммы');
  vm.startEdit(line.id);
  expect(await vm.saveEdit({ title: 'Аванс в мае', amount: 1_500_000 })).toBe(true);
  expect(vm.editingId).toBeNull();
  expect(vm.cards[0]?.amounts[0]).toMatchObject({ title: 'Аванс в мае', amount: 1_500_000 });
  expect(vm.monthTotal).toBe(2_500_000);
  await undoStack.undo();
  expect(vm.cards[0]?.amounts[0]).toMatchObject({ title: 'Аванс', amount: 1_000_000 });
  expect(vm.monthTotal).toBe(2_000_000);
});

it('правка без изменений ничего не отправляет; нулевая сумма отклоняется', async () => {
  const vm = await opened();
  await put(vm, vm.cards[0]?.categoryId ?? 0, 1_000_000, 'Аванс');
  const line = vm.cards[0]?.amounts[0];
  if (!line) throw new Error('нет суммы');
  vm.startEdit(line.id);
  const depth = undoStack.depth;
  expect(await vm.saveEdit({ title: 'Аванс', amount: 1_000_000 })).toBe(true);
  expect(undoStack.depth).toBe(depth);
  vm.startEdit(line.id);
  expect(await vm.saveEdit({ title: 'Аванс', amount: 0 })).toBe(false);
  expect(toasts.items.at(-1)?.kind).toBe('error');
  expect(vm.cards[0]?.amounts[0]?.amount).toBe(1_000_000);
});

it('сумму можно удалить: итог уменьшается, тост «Вернуть» возвращает её', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await put(vm, id, 1_000_000, 'Аванс');
  await put(vm, id, 2_000_000, 'Премия');
  const first = vm.cards[0]?.amounts[0];
  if (!first) throw new Error('нет суммы');
  await vm.remove(first.id);
  expect(vm.cards[0]?.amounts.map((t) => t.title)).toEqual(['Премия']);
  expect(vm.monthTotal).toBe(2_000_000);
  const toast = toasts.items.at(-1);
  expect(toast?.message).toBe('Сумма удалена');
  expect(toast?.action?.label).toBe('Вернуть');
  toasts.runAction(toast?.id ?? 0);
  await vi.waitFor(() => {
    expect(vm.monthTotal).toBe(3_000_000);
  });
  expect(vm.cards[0]?.amounts).toHaveLength(2);
});

it('добавление суммы отменяется ⌘Z, повтор возвращает ту же строку', async () => {
  const vm = await opened();
  await put(vm, vm.cards[0]?.categoryId ?? 0, 700_000);
  expect(vm.monthTotal).toBe(700_000);
  await undoStack.undo();
  expect(vm.monthTotal).toBe(0);
  await undoStack.redo();
  expect(vm.monthTotal).toBe(700_000);
});

it('плановые строки старых данных не попадают в суммы карточки', async () => {
  // В моке сентябрь 2026 содержит плановый «Вклад» в накоплениях.
  const vm = await opened('2026-09');
  expect(vm.cards[0]?.amounts.every((t) => t.status !== 'planned')).toBe(true);
});

it('новое накопление создаётся категорией «Сбережения» и появляется второй карточкой', async () => {
  const vm = await opened();
  const before = vm.cards.length;
  vm.newName = 'Отпуск';
  expect(await vm.add()).toBe(true);
  expect(vm.newName).toBe('');
  expect(vm.cards).toHaveLength(before + 1);
  expect(vm.cards.map((c) => c.name)).toContain('Отпуск');
});

it('пустое имя накопления не создаётся', async () => {
  const vm = await opened();
  vm.newName = '   ';
  expect(await vm.add()).toBe(false);
});

it('план месяца задаётся по накоплению и отражается в карточке и итоге плана', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  expect(await vm.setPlan(id, 2_000_000)).toBe(true);
  expect(vm.cards[0]?.plan).toBe(2_000_000);
  expect(vm.planTotal).toBe(2_000_000);
});

it('накоплено и помесячные взносы приходят из Rust и меняются вместе с суммами', async () => {
  const vm = await opened();
  const before = vm.progress?.total ?? 0;
  await put(vm, vm.cards[0]?.categoryId ?? 0, 700_000);
  await put(vm, vm.cards[0]?.categoryId ?? 0, 300_000);
  expect(vm.progress?.total).toBe(before + 1_000_000);
});

// ---- Q-18: закрыть цель, график по всем накоплениям ----

it('закрытие цели спрашивает подтверждение: карточка остаётся, пока не подтверждено', async () => {
  const vm = await opened();
  const card = vm.cards[0];
  if (!card) throw new Error('нет накопления');
  vm.askClose(card.categoryId);
  expect(vm.closeAsk).toEqual({ id: card.categoryId, name: 'Накопления' });
  expect(vm.cards).toHaveLength(1);
  vm.cancelClose();
  expect(vm.closeAsk).toBeNull();
  expect(vm.cards).toHaveLength(1);
});

it('подтверждённая цель закрывается: карточки нет, она в списке закрытых и возвращается', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await put(vm, id, 1_000_000, 'Аванс');
  vm.askClose(id);
  await vm.confirmClose();
  expect(vm.closeAsk).toBeNull();
  expect(vm.cards).toHaveLength(0);
  expect(vm.closed.map((g) => g.name)).toEqual(['Накопления']);
  expect(toasts.items.at(-1)?.message).toBe('Цель закрыта');
  await vm.reopen(id);
  expect(vm.closed).toHaveLength(0);
  expect(vm.cards.map((c) => c.categoryId)).toEqual([id]);
});

it('суммы закрытой цели остаются в «Накоплено всего», «Отложено в месяце» и в графике', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await put(vm, id, 1_000_000);
  const before = vm.progress?.total ?? 0;
  expect(vm.monthTotal).toBe(1_000_000);
  vm.askClose(id);
  await vm.confirmClose();
  expect(vm.progress?.total).toBe(before);
  expect(vm.monthTotal).toBe(1_000_000);
  const goal = vm.progress?.categories.find((c) => c.categoryId === id);
  expect(goal).toMatchObject({ archived: true });
  expect(vm.progress?.chart.series.some((s) => s.color === `category:${String(id)}`)).toBe(true);
  expect(vm.closed[0]?.total).toBe(goal?.total);
});

it('график: ряд на каждое накопление, цвета берутся из данных, в том числе у закрытых', async () => {
  const vm = await opened();
  const first = vm.cards[0]?.categoryId ?? 0;
  vm.newName = 'Отпуск';
  await vm.add();
  const second = vm.cards.find((c) => c.name === 'Отпуск')?.categoryId ?? 0;
  await put(vm, first, 1_000_000);
  await put(vm, second, 2_000_000);
  vm.askClose(second);
  await vm.confirmClose();
  const series = vm.progress?.chart.series ?? [];
  const march = (vm.progress?.chart.categoryTokens ?? []).indexOf(`month:${MONTH}`);
  expect(series.map((s) => s.name).sort()).toEqual(['Накопления', 'Отпуск']);
  expect(march).toBeGreaterThanOrEqual(0);
  const byName = new Map(series.map((s) => [s.name, s.values[march]]));
  expect(byName.get('Накопления')).toBe(10_000);
  expect(byName.get('Отпуск')).toBe(20_000);
  expect(vm.chartColors.get(second)).toBeDefined();
  expect(vm.chartColors.get(first)).toBeDefined();
});

it('новые накопления получают разные цвета, чтобы ряды графика различались', async () => {
  const vm = await opened();
  for (const name of ['Отпуск', 'Машина', 'Ремонт']) {
    vm.newName = name;
    await vm.add();
  }
  const colors = (await categoriesApi.list(true))
    .filter((c) => c.kind === 'savings')
    .map((c) => c.color);
  expect(colors).toHaveLength(4);
  expect(new Set(colors).size).toBe(4);
});

it('цвет не повторяет цвет закрытой цели', async () => {
  const vm = await opened();
  vm.askClose(vm.cards[0]?.categoryId ?? 0);
  await vm.confirmClose();
  vm.newName = 'Отпуск';
  await vm.add();
  const colors = (await categoriesApi.list(true))
    .filter((c) => c.kind === 'savings')
    .map((c) => c.color);
  expect(new Set(colors).size).toBe(colors.length);
});

// ---- переименование (Issue #137) ----

const nameOf = async (id: number): Promise<string | undefined> =>
  (await categoriesApi.list(true)).find((c) => c.id === id)?.name;

async function withSecondGoal(): Promise<SavingsVm> {
  const vm = await opened();
  vm.newName = 'Отпуск';
  await vm.add();
  return vm;
}

it('переименование: имя меняется в карточке и в графике, тост «Переименовано» с «Отменить»', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await put(vm, id, 500_000);
  vm.startRename(id);
  expect(vm.renamingId).toBe(id);
  expect(await vm.rename(id, '  Подушка  ')).toBe(true);
  expect(vm.renamingId).toBeNull();
  expect(vm.cards[0]?.name).toBe('Подушка');
  expect(await nameOf(id)).toBe('Подушка');
  expect(vm.progress?.chart.series.map((s) => s.name)).toContain('Подушка');
  expect(categories.items.find((c) => c.id === id)?.name).toBe('Подушка');
  const toast = toasts.items.at(-1);
  expect(toast?.message).toBe('Переименовано: «Подушка»');
  expect(toast?.action?.label).toBe('Отменить');
});

it('переименование: то же имя закрывает поле и ничего не пишет', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  vm.startRename(id);
  expect(await vm.rename(id, 'Накопления')).toBe(true);
  expect(vm.renamingId).toBeNull();
  expect(toasts.items).toHaveLength(0);
  expect(undoStack.canUndo).toBe(false);
});

it('переименование: пустое имя отклоняется, прежнее остаётся, тост с ошибкой', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  vm.startRename(id);
  expect(await vm.rename(id, '   ')).toBe(false);
  expect(vm.renamingId).toBeNull();
  expect(vm.cards[0]?.name).toBe('Накопления');
  expect(toasts.items.at(-1)).toMatchObject({
    kind: 'error',
    message: 'Укажите название категории.'
  });
  expect(undoStack.canUndo).toBe(false);
});

it('переименование: занятое имя (без учёта регистра и ё/е) отклоняется, тост по-русски', async () => {
  const vm = await withSecondGoal();
  const id = vm.cards[0]?.categoryId ?? 0;
  expect(await vm.rename(id, 'отпуск')).toBe(false);
  expect(vm.cards[0]?.name).toBe('Накопления');
  expect(toasts.items.at(-1)).toMatchObject({
    kind: 'error',
    message: 'Категория с таким названием уже есть.'
  });
  expect(undoStack.canUndo).toBe(false);
});

it('переименование: ⌘Z возвращает прежнее имя, повтор применяет новое', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await vm.rename(id, 'Подушка');
  await undoStack.undo();
  expect(vm.cards[0]?.name).toBe('Накопления');
  expect(await nameOf(id)).toBe('Накопления');
  await undoStack.redo();
  expect(vm.cards[0]?.name).toBe('Подушка');
  expect(await nameOf(id)).toBe('Подушка');
});

it('переименование: действие «Отменить» из тоста возвращает прежнее имя', async () => {
  const vm = await opened();
  const id = vm.cards[0]?.categoryId ?? 0;
  await vm.rename(id, 'Подушка');
  toasts.items.at(-1)?.action?.run();
  await vi.waitFor(() => {
    expect(vm.cards[0]?.name).toBe('Накопления');
  });
});

it('переименование: Esc (cancelRename) закрывает поле без записи', async () => {
  const vm = await opened();
  vm.startRename(vm.cards[0]?.categoryId ?? 0);
  vm.cancelRename();
  expect(vm.renamingId).toBeNull();
  expect(vm.cards[0]?.name).toBe('Накопления');
});

it('переименование: закрытая цель не переименовывается', async () => {
  const vm = await withSecondGoal();
  const id = vm.cards[0]?.categoryId ?? 0;
  vm.askClose(id);
  await vm.confirmClose();
  vm.startRename(id);
  expect(vm.renamingId).toBeNull();
  expect(await vm.rename(id, 'Другое')).toBe(false);
  expect(await nameOf(id)).toBe('Накопления');
});

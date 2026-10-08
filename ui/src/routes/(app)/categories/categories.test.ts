import { emit } from '@tauri-apps/api/event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { COALESCE_MS } from '$lib/api/data-events';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categoriesApi } from '$lib/api/data';
import { parsePercentBp } from '$lib/category-colors';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { CategoriesVm } from './categories.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

it('load: строки с лимитом месяца и процентом сбережений', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  expect(vm.rows).toHaveLength(4);
  expect(vm.rows.find((r) => r.category.name === 'Продукты')?.limit).toBe(3_000_000);
  expect(vm.rows.find((r) => r.category.kind === 'savings')?.rateBp).toBe(1400);
  expect(vm.monthPlanBp).toBe(1400);
  expect(vm.rateMismatch).toBe(false);
});

it('сохранение нового лимита действует с начала месяца экрана', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.select(1);
  vm.draft.limit = 3_500_000;
  expect(await vm.save()).toBe(true);
  expect(vm.rows.find((r) => r.category.id === 1)?.limit).toBe(3_500_000);
  await vm.load('2026-08');
  expect(vm.rows.find((r) => r.category.id === 1)?.limit).toBe(3_000_000);
});

it('пустое поле лимита убирает его с начала месяца, прошлые месяцы не меняются', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.select(1);
  vm.draft.limit = null;
  expect(await vm.save()).toBe(true);
  expect(vm.rows.find((r) => r.category.id === 1)?.limit).toBeNull();
  await vm.load('2026-08');
  expect(vm.rows.find((r) => r.category.id === 1)?.limit).toBe(3_000_000);
});

it('пустой процент сбережений обнуляет план с начала месяца', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.select(4);
  expect(vm.rows.find((r) => r.category.id === 4)?.rateBp).toBe(1400);
  vm.draft.rate = '';
  expect(await vm.save()).toBe(true);
  expect(vm.rows.find((r) => r.category.id === 4)?.rateBp).toBe(0);
});

it('пустое название не сохраняется', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.select(1);
  vm.draft.name = '  ';
  expect(await vm.save()).toBe(false);
  expect(vm.fieldError).toBe('Название не должно быть пустым');
});

it('создание категории добавляет её в конец', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.startCreate();
  vm.draft.name = 'Спорт';
  vm.draft.limit = 500_000;
  expect(await vm.save()).toBe(true);
  expect(vm.rows.at(-1)?.category.name).toBe('Спорт');
  expect(vm.rows.at(-1)?.limit).toBe(500_000);
});

it('архив скрывает категорию, «показать архив» возвращает', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  await vm.setArchived(3, true);
  expect(vm.rows.map((r) => r.category.id)).not.toContain(3);
  vm.showArchived = true;
  expect(vm.rows.map((r) => r.category.id)).toContain(3);
});

it('move переставляет категории', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  await vm.move(4, 1);
  expect(vm.items.map((c) => c.id)).toEqual([4, 1, 2, 3]);
});

it('процент сбережений: расхождение с нормой помечается', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.select(4);
  vm.draft.rate = '18';
  expect(await vm.save()).toBe(true);
  expect(vm.monthPlanBp).toBe(1800);
  expect(vm.rateMismatch).toBe(true);
});

it('parsePercentBp', () => {
  expect(parsePercentBp('14')).toBe(1400);
  expect(parsePercentBp('14,5 %')).toBe(1450);
  expect(parsePercentBp('101')).toBeNull();
  expect(parsePercentBp('abc')).toBeNull();
});

it('лимит и процент уходят в Rust датой «ГГГГ-ММ» (YearMonth::parse)', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.select(1);
  vm.draft.limit = 3_100_000;
  await vm.save();
  const history = await categoriesApi.limitHistory(1);
  expect(history.map((e) => e.validFrom)).toContain('2026-09');
});

it('неверный процент месяца не создаёт категорию', async () => {
  const vm = new CategoriesVm();
  await vm.load('2026-09');
  vm.startCreate();
  vm.draft.name = 'Копилка';
  vm.draft.kind = 'savings';
  vm.draft.monthRate = 'abc';
  expect(await vm.save()).toBe(false);
  await vm.load('2026-09');
  expect(vm.items.map((c) => c.name)).not.toContain('Копилка');
});

it('сохранение с несколькими командами не вызывает лишних перечитываний по событиям', async () => {
  const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));
  const vm = new CategoriesVm();
  const off = vm.connect();
  await vm.load('2026-09');
  await settle(20);
  vm.select(1);
  vm.draft.name = 'Продукты и дом';
  vm.draft.limit = 3_500_000;
  const list = vi.spyOn(categoriesApi, 'list');
  // Событие первой команды сбрасывается по таймеру, пока вторая ещё выполняется.
  const update = categoriesApi.update.bind(categoriesApi);
  vi.spyOn(categoriesApi, 'update').mockImplementation(async (...args) => {
    const result = await update(...args);
    await emit('data-changed', { scope: 'categories', months: [] });
    await settle(COALESCE_MS * 2);
    return result;
  });
  expect(await vm.save()).toBe(true);
  await settle(COALESCE_MS * 3);
  expect(list).toHaveBeenCalledTimes(1);
  off();
  vi.restoreAllMocks();
});

it('стор активных категорий: события во время загрузки дают одну повторную', async () => {
  const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));
  await categories.load();
  const off = categories.watch();
  await settle(20);
  const list = categoriesApi.list.bind(categoriesApi);
  const spy = vi.spyOn(categoriesApi, 'list').mockImplementation(async (...args) => {
    await settle(COALESCE_MS * 4);
    return list(...args);
  });
  for (let i = 0; i < 3; i++) {
    await emit('data-changed', { scope: 'categories', months: [] });
    await settle(COALESCE_MS * 1.5);
  }
  await settle(COALESCE_MS * 12);
  expect(spy).toHaveBeenCalledTimes(2);
  off();
  vi.restoreAllMocks();
});

it('сбой на середине сохранения: экран перечитывается и повторное сохранение обновляет созданную категорию', async () => {
  const vm = new CategoriesVm();
  const off = vm.connect();
  await vm.load('2026-09');
  vm.startCreate();
  vm.draft.name = 'Хобби';
  vm.draft.kind = 'wants';
  vm.draft.limit = 100_000;
  const setLimit = vi.spyOn(categoriesApi, 'setLimit').mockRejectedValueOnce(new Error('boom'));
  expect(await vm.save()).toBe(false);
  await new Promise((r) => setTimeout(r, 30));
  expect(vm.items.some((c) => c.name === 'Хобби')).toBe(true);

  expect(await vm.save()).toBe(true);
  expect(vm.items.filter((c) => c.name === 'Хобби')).toHaveLength(1);
  setLimit.mockRestore();
  off();
});

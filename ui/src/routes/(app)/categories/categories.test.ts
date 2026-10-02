import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
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

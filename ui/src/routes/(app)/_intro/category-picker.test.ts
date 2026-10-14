import { afterEach, expect, it, vi } from 'vitest';
import type { CategoryDto, DefaultCategoryDto } from '$lib/api/bindings';
import { categoriesApi } from '$lib/api/data';
import { CategoryPickerVm } from './category-picker.svelte';

const DEFAULTS: DefaultCategoryDto[] = [
  { name: 'Продукты', kind: 'mandatory', required: false },
  { name: 'Подписки', kind: 'wants', required: false },
  { name: 'Прочее', kind: 'wants', required: true },
  { name: 'Сбережения', kind: 'savings', required: true }
];

afterEach(() => {
  vi.restoreAllMocks();
});

function fresh(): CategoryPickerVm {
  vi.spyOn(categoriesApi, 'list').mockResolvedValue([]);
  vi.spyOn(categoriesApi, 'defaults').mockResolvedValue(DEFAULTS);
  return new CategoryPickerVm();
}

it('пустое хранилище: шаг нужен, по умолчанию отмечены все категории, группы по видам', async () => {
  const vm = fresh();
  await vm.load();
  expect(vm.ready).toBe(true);
  expect(vm.needed).toBe(true);
  expect(vm.chosen.size).toBe(4);
  expect(vm.groups.map((g) => g.label)).toEqual(['Обязательные', 'Желания', 'Сбережения']);
});

it('категории уже есть (знакомство показано повторно): шага нет, ничего не создаётся', async () => {
  vi.spyOn(categoriesApi, 'list').mockResolvedValue([{ id: 1 } as CategoryDto]);
  const defaults = vi.spyOn(categoriesApi, 'defaults');
  const seed = vi.spyOn(categoriesApi, 'seed');
  const vm = new CategoryPickerVm();
  await vm.load();
  expect(vm.needed).toBe(false);
  expect(await vm.apply()).toBe(true);
  expect(defaults).not.toHaveBeenCalled();
  expect(seed).not.toHaveBeenCalled();
});

it('«Только обязательные» оставляет закреплённые, переключатель меняет одну категорию', async () => {
  const vm = fresh();
  await vm.load();
  vm.setAll(false);
  expect([...vm.chosen].sort()).toEqual(['Прочее', 'Сбережения']);
  vm.toggle('Подписки');
  expect(vm.isChosen('Подписки')).toBe(true);
  vm.toggle('Подписки');
  expect(vm.isChosen('Подписки')).toBe(false);
  vm.setAll(true);
  expect(vm.chosen.size).toBe(4);
});

it('apply отправляет отмеченные названия и закрывает шаг', async () => {
  const vm = fresh();
  const seed = vi.spyOn(categoriesApi, 'seed').mockResolvedValue([]);
  await vm.load();
  vm.setAll(false);
  vm.toggle('Продукты');
  expect(await vm.apply()).toBe(true);
  expect([...(seed.mock.calls[0]?.[0] ?? [])].sort()).toEqual(['Продукты', 'Прочее', 'Сбережения']);
  expect(vm.needed).toBe(false);
});

it('архивные категории считаются: список запрашивается вместе с ними', async () => {
  const vm = fresh();
  await vm.load();
  expect(categoriesApi.list).toHaveBeenCalledWith(true);
});

it('сбой чтения при открытии: при выходе чтение повторяется один раз', async () => {
  const list = vi.spyOn(categoriesApi, 'list').mockRejectedValueOnce(new Error('locked'));
  vi.spyOn(categoriesApi, 'defaults').mockResolvedValue(DEFAULTS);
  const seed = vi.spyOn(categoriesApi, 'seed').mockResolvedValue([]);
  const vm = new CategoryPickerVm();
  await vm.load();
  expect(vm.ready).toBe(true);
  expect(vm.needed).toBe(false);
  list.mockResolvedValue([]);
  expect(await vm.apply()).toBe(true);
  expect(seed).toHaveBeenCalledTimes(1);
});

it('ошибка сохранения: шаг остаётся, текст ошибки показан', async () => {
  const vm = fresh();
  vi.spyOn(categoriesApi, 'seed').mockRejectedValue(new Error('boom'));
  await vm.load();
  expect(await vm.apply()).toBe(false);
  expect(vm.needed).toBe(true);
  expect(vm.error).not.toBeNull();
  expect(vm.busy).toBe(false);
});

it('чтение категорий не удалось дважды: выход не засчитывается и ничего не создаётся', async () => {
  vi.spyOn(categoriesApi, 'list').mockRejectedValue(new Error('locked'));
  const seed = vi.spyOn(categoriesApi, 'seed');
  const vm = new CategoryPickerVm();
  await vm.load();
  expect(await vm.apply()).toBe(false);
  expect(vm.error).not.toBeNull();
  expect(vm.busy).toBe(false);
  expect(seed).not.toHaveBeenCalled();
});

it('второй выход во время первого не запускает повторный apply', async () => {
  const vm = fresh();
  await vm.load();
  let release: (value: CategoryDto[]) => void = () => undefined;
  const seed = vi.spyOn(categoriesApi, 'seed').mockReturnValue(
    new Promise((resolve) => {
      release = resolve;
    })
  );
  const first = vm.apply();
  expect(await vm.apply()).toBe(false);
  release([]);
  expect(await first).toBe(true);
  expect(seed).toHaveBeenCalledTimes(1);
});

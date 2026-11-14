import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { filtersApi, transactionsApi } from '$lib/api/data';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { tags } from '$lib/stores/tags.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { TagsVm, deleteHint, renameHint, usesText } from './tags-settings.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
  tags.reset();
});

afterEach(() => {
  clearMocks();
  tags.reset();
  undoStack.clear();
});

it('usesText, deleteHint и renameHint: склонение и предупреждения', () => {
  expect(usesText(1)).toBe('1 трата');
  expect(usesText(3)).toBe('3 траты');
  expect(usesText(11)).toBe('11 трат');
  const tag = { id: 1, name: 'x', uses: 0, refs: 0 };
  expect(deleteHint(tag)).toBe('Тег нигде не используется. Вернуть тег нельзя.');
  expect(deleteHint({ ...tag, uses: 2 })).toContain('2 траты');
  expect(deleteHint({ ...tag, refs: 1 })).toContain('1 сохранённый запрос');
  expect(renameHint(tag)).toBe('');
  expect(renameHint({ ...tag, refs: 3 })).toContain('3 сохранённых запроса');
});

it('load: список тегов с числом трат', async () => {
  const vm = new TagsVm();
  await vm.load();
  expect(vm.loaded).toBe(true);
  expect(vm.items.map((t) => t.name)).toEqual(['Подарки', 'Работа']);
});

it('rename: имя меняется в списке, занятое отклоняется с текстом ошибки', async () => {
  const vm = new TagsVm();
  await vm.load();
  const gifts = vm.items.find((t) => t.name === 'Подарки');
  if (!gifts) throw new Error('нет тега');

  expect(await vm.rename(gifts.id, 'Праздники')).toBe(true);
  expect(vm.items.map((t) => t.name)).toEqual(['Праздники', 'Работа']);
  expect(toasts.items.some((t) => t.message.includes('Праздники'))).toBe(true);

  expect(await vm.rename(gifts.id, 'работа')).toBe(false);
  expect(vm.error).toBe('Тег с таким названием уже есть.');
  expect(vm.items.find((t) => t.id === gifts.id)?.name).toBe('Праздники');
});

it('remove: тег исчезает из списка и снимается со всех трат', async () => {
  const vm = new TagsVm();
  await vm.load();
  const tag = vm.items[0];
  if (!tag) throw new Error('нет тегов');
  const month = '2026-09';
  const rows = await transactionsApi.list(month);
  const free = rows.find((t) => !t.tagIds.includes(tag.id));
  if (!free) throw new Error('нет траты без тега');
  const before = tag.uses;
  await transactionsApi.setTags(free.id, [...free.tagIds, tag.id]);
  await vm.load();
  const loaded = vm.items.find((t) => t.id === tag.id);
  expect(loaded?.uses).toBe(before + 1);

  expect(await vm.remove(loaded ?? tag)).toBe(true);

  expect(vm.items.some((t) => t.id === tag.id)).toBe(false);
  const after = await transactionsApi.list(month);
  expect(after.some((t) => t.tagIds.includes(tag.id))).toBe(false);
});

it('rename переписывает имя в сохранённом фильтре; remove вычёркивает записи отмены про тег', async () => {
  await filtersApi.save('Праздники за год', '#подарки кат:продукты', 'expenses');
  const vm = new TagsVm();
  await vm.load();
  const gifts = vm.items.find((t) => t.name === 'Подарки');
  if (!gifts) throw new Error('нет тега');
  expect(gifts.refs).toBe(1);

  expect(await vm.rename(gifts.id, 'Дача')).toBe(true);
  expect((await filtersApi.list())[0]?.query).toBe('#Дача кат:продукты');
  expect(vm.items.find((t) => t.id === gifts.id)?.refs).toBe(1);

  undoStack.record({
    label: 'смена тегов',
    undo: () => undefined,
    redo: () => undefined,
    tags: [gifts.id]
  });
  const renamed = vm.items.find((t) => t.id === gifts.id);
  if (!renamed) throw new Error('нет тега после переименования');
  expect(await vm.remove(renamed)).toBe(true);
  expect(undoStack.depth).toBe(0);
});

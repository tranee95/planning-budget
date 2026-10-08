import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { savedFilters } from './saved-filters.svelte';
import { toasts } from './toasts.svelte';
import { undoStack } from './undo.svelte';

beforeEach(async () => {
  installMocks();
  resetMockBudget();
  toasts.clear();
  undoStack.clear();
  savedFilters.reset();
  await savedFilters.load();
});

afterEach(() => {
  clearMocks();
  savedFilters.reset();
});

const find = (name: string) => savedFilters.items.find((f) => f.name === name);

it('save нового фильтра: отмена удаляет его, повтор возвращает', async () => {
  await savedFilters.save('Крупные', 'сумма>5000', 'expenses');
  expect(undoStack.depth).toBe(1);

  await undoStack.undo();
  expect(find('Крупные')).toBeUndefined();
  await undoStack.redo();
  expect(find('Крупные')?.query).toBe('сумма>5000');
});

it('save поверх существующего имени: отмена возвращает прежний запрос', async () => {
  await savedFilters.save('Крупные', 'сумма>5000', 'expenses');
  await savedFilters.save('Крупные', 'сумма>9000', 'expenses');
  expect(find('Крупные')?.query).toBe('сумма>9000');

  await undoStack.undo();
  expect(find('Крупные')?.query).toBe('сумма>5000');
  await undoStack.redo();
  expect(find('Крупные')?.query).toBe('сумма>9000');
});

it('remove: отмена возвращает удалённый фильтр, повтор удаляет снова', async () => {
  const saved = await savedFilters.save('Крупные', 'сумма>5000', 'expenses');
  undoStack.clear();

  await savedFilters.remove(saved.id);
  expect(find('Крупные')).toBeUndefined();
  expect(undoStack.depth).toBe(1);

  await undoStack.undo();
  expect(find('Крупные')?.query).toBe('сумма>5000');
  await undoStack.redo();
  expect(find('Крупные')).toBeUndefined();
});

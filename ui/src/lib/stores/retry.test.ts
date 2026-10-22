import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api/call';
import { incomesApi } from '$lib/api/data';
import { isRetryable } from '$lib/i18n/errors';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { IncomesVm } from '../../routes/(app)/incomes/incomes.svelte';
import { toasts } from './toasts.svelte';
import { undoStack } from './undo.svelte';

const busy = (): ApiError => new ApiError({ code: 'Conflict', messageKey: 'errors.db.busy' });

beforeEach(() => {
  installMocks();
  resetMockBudget();
  toasts.clear();
  undoStack.clear();
});

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
});

it('повтор нужен только при занятой базе и внутреннем сбое', () => {
  expect(isRetryable(busy())).toBe(true);
  expect(isRetryable(new ApiError({ code: 'Internal', correlationId: 'c' }))).toBe(true);
  expect(
    isRetryable(new ApiError({ code: 'Validation', messageKey: 'errors.x', field: null }))
  ).toBe(false);
  expect(isRetryable(new ApiError({ code: 'Locked' }))).toBe(false);
  expect(isRetryable(new Error('boom'))).toBe(false);
});

it('смена статуса дохода: при занятой базе тост с «Повторить», повтор применяет статус', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  const income = vm.incomes.find((i) => i.status === 'received');
  expect(income).toBeDefined();
  if (income === undefined) return;
  vi.spyOn(incomesApi, 'update').mockRejectedValueOnce(busy());

  await vm.setStatus(income.id, 'expected');
  expect(vm.incomes.find((i) => i.id === income.id)?.status).toBe('received');
  const toast = toasts.items.at(-1);
  expect(toast?.kind).toBe('error');
  expect(toast?.action?.label).toBe('Повторить');

  toast?.action?.run();
  await vi.waitFor(() => {
    expect(vm.incomes.find((i) => i.id === income.id)?.status).toBe('expected');
  });
});

it('ошибка ввода повтором не лечится: кнопки «Повторить» нет', async () => {
  const vm = new IncomesVm();
  await vm.load('2026-09');
  const income = vm.incomes[0];
  if (income === undefined) return;
  vi.spyOn(incomesApi, 'update').mockRejectedValueOnce(
    new ApiError({
      code: 'Validation',
      messageKey: 'errors.record.amount_not_positive',
      field: null
    })
  );
  await vm.setStatus(income.id, income.status === 'received' ? 'expected' : 'received');
  expect(toasts.items.at(-1)?.action).toBeUndefined();
});

it('повтор неудавшейся отмены проходит через undo(): стек и тост «Отменено» обновляются', async () => {
  let attempts = 0;
  const entry = {
    label: 'тест',
    undo: () => {
      attempts++;
      if (attempts === 1) throw busy();
    },
    redo: () => undefined
  };
  undoStack.record(entry);
  await undoStack.undo();
  expect(undoStack.depth).toBe(1);
  toasts.items.at(-1)?.action?.run();
  await vi.waitFor(() => {
    expect(undoStack.depth).toBe(0);
  });
  expect(undoStack.canRedo).toBe(true);
  expect(toasts.items.some((t) => t.message.startsWith('Отменено'))).toBe(true);
});

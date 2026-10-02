import { afterEach, expect, it, vi } from 'vitest';
import { session } from './session.svelte';
import { toasts } from './toasts.svelte';
import { undoStack } from './undo.svelte';

afterEach(() => {
  undoStack.clear();
  toasts.clear();
});

const entry = (label = 'Действие') => ({ label, undo: vi.fn(), redo: vi.fn() });

it('undo вызывает откат последней записи, redo — повтор', async () => {
  const a = entry();
  undoStack.record(a);
  expect(undoStack.canUndo).toBe(true);
  await undoStack.undo();
  expect(a.undo).toHaveBeenCalledOnce();
  expect(undoStack.canUndo).toBe(false);
  expect(undoStack.canRedo).toBe(true);
  await undoStack.redo();
  expect(a.redo).toHaveBeenCalledOnce();
  expect(undoStack.canRedo).toBe(false);
});

it('новая запись сбрасывает redo', async () => {
  undoStack.record(entry('a'));
  await undoStack.undo();
  undoStack.record(entry('b'));
  expect(undoStack.canRedo).toBe(false);
});

it('на пустом стеке ничего не делает', async () => {
  await undoStack.undo();
  await undoStack.redo();
  expect(toasts.items).toHaveLength(0);
});

it('хранит не больше 50 записей', () => {
  for (let i = 0; i < 60; i++) undoStack.record(entry(String(i)));
  expect(undoStack.depth).toBe(50);
});

it('ошибка отката остаётся в стеке и сообщается тостом', async () => {
  const a = { label: 'a', undo: vi.fn().mockRejectedValue(new Error('x')), redo: vi.fn() };
  undoStack.record(a);
  await undoStack.undo();
  expect(toasts.items.at(-1)?.kind).toBe('error');
  expect(undoStack.canUndo).toBe(true);
});

it('блокировка очищает стек', () => {
  undoStack.record(entry());
  session.lockedFromBackend();
  expect(undoStack.canUndo).toBe(false);
});

it('undo(запись) отменяет именно её, а не последнюю', async () => {
  const a = entry('a');
  const b = entry('b');
  undoStack.record(a);
  undoStack.record(b);
  await undoStack.undo(a);
  expect(a.undo).toHaveBeenCalledOnce();
  expect(b.undo).not.toHaveBeenCalled();
  expect(undoStack.depth).toBe(1);
});

it('undo(запись), которой нет в стеке, ничего не делает', async () => {
  const a = entry('a');
  await undoStack.undo(a);
  expect(a.undo).not.toHaveBeenCalled();
});

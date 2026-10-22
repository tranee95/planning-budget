import { expect, it, vi } from 'vitest';
import { deleteOnKey } from './row-delete';

function press(target: HTMLElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'target', { value: target });
  return event;
}

it('Delete на кнопке строки удаляет её', () => {
  const remove = vi.fn();
  const event = press(document.createElement('button'), 'Delete');
  deleteOnKey(event, remove);
  expect(remove).toHaveBeenCalledOnce();
  expect(event.defaultPrevented).toBe(true);
});

it('в поле ввода Delete стирает символ, а не строку', () => {
  const remove = vi.fn();
  deleteOnKey(press(document.createElement('input'), 'Delete'), remove);
  expect(remove).not.toHaveBeenCalled();
});

it('другие клавиши ничего не удаляют', () => {
  const remove = vi.fn();
  deleteOnKey(press(document.createElement('button'), 'Backspace'), remove);
  expect(remove).not.toHaveBeenCalled();
});

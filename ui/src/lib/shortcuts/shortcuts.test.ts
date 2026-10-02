import { describe, expect, it, vi } from 'vitest';
import { createShortcuts, matches, type Shortcut } from './index';

function key(init: KeyboardEventInit & { target?: Element }): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
  if (init.target) Object.defineProperty(event, 'target', { value: init.target });
  return event;
}

const run = vi.fn();
const shortcut = (over: Partial<Shortcut>): Shortcut => ({
  id: 'x',
  code: 'KeyN',
  label: 'Новая трата',
  run,
  ...over
});

describe('matches', () => {
  it('сопоставляет по коду клавиши, а не по символу', () => {
    expect(matches(shortcut({}), key({ code: 'KeyN', key: 'т' }))).toBe(true);
    expect(matches(shortcut({}), key({ code: 'KeyI', key: 'n' }))).toBe(false);
  });

  it('Mod — это Ctrl или Cmd', () => {
    const s = shortcut({ code: 'KeyK', mod: true });
    expect(matches(s, key({ code: 'KeyK', ctrlKey: true }))).toBe(true);
    expect(matches(s, key({ code: 'KeyK', metaKey: true }))).toBe(true);
    expect(matches(s, key({ code: 'KeyK' }))).toBe(false);
  });

  it('клавиша без Mod не срабатывает с Ctrl', () => {
    expect(matches(shortcut({}), key({ code: 'KeyN', ctrlKey: true }))).toBe(false);
  });
});

describe('createShortcuts', () => {
  it('не срабатывает в полях ввода, кроме клавиш с inInput', () => {
    run.mockClear();
    const input = document.createElement('input');
    const registry = createShortcuts();
    registry.register(shortcut({ id: 'n' }));
    registry.register(shortcut({ id: 'k', code: 'KeyK', mod: true, inInput: true }));

    registry.handle(key({ code: 'KeyN', target: input }));
    expect(run).not.toHaveBeenCalled();
    registry.handle(key({ code: 'KeyK', ctrlKey: true, target: input }));
    expect(run).toHaveBeenCalledTimes(1);
  });

  it('старая регистрация, снятая позже новой с тем же id, не стирает новую (outro экрана)', () => {
    run.mockClear();
    const registry = createShortcuts();
    const unregisterOld = registry.register(shortcut({ id: 'n' }));
    registry.register(shortcut({ id: 'n' }));
    unregisterOld();
    registry.handle(key({ code: 'KeyN' }));
    expect(run).toHaveBeenCalledTimes(1);
  });

  it('снимает регистрацию', () => {
    run.mockClear();
    const registry = createShortcuts();
    const off = registry.register(shortcut({}));
    off();
    registry.handle(key({ code: 'KeyN' }));
    expect(run).not.toHaveBeenCalled();
  });

  it('вызывает preventDefault для сработавшей клавиши', () => {
    const registry = createShortcuts();
    registry.register(shortcut({}));
    const event = key({ code: 'KeyN' });
    registry.handle(event);
    expect(event.defaultPrevented).toBe(true);
  });
});

describe('диалоги', () => {
  it('одиночные клавиши не срабатывают, пока открыт диалог', () => {
    run.mockClear();
    const registry = createShortcuts();
    registry.register(shortcut({ id: 'n' }));
    registry.register(shortcut({ id: 'k', code: 'KeyK', mod: true, inInput: true }));
    const dialog = document.createElement('div');
    dialog.setAttribute('aria-modal', 'true');
    document.body.append(dialog);

    registry.handle(key({ code: 'KeyN' }));
    expect(run).not.toHaveBeenCalled();
    registry.handle(key({ code: 'KeyK', ctrlKey: true }));
    expect(run).toHaveBeenCalledTimes(1);

    dialog.remove();
    registry.handle(key({ code: 'KeyN' }));
    expect(run).toHaveBeenCalledTimes(2);
  });
});

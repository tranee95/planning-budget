import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { session } from './session.svelte';
import { toasts } from './toasts.svelte';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  toasts.clear();
  vi.useRealTimers();
});

describe('toasts', () => {
  it('исчезает через 5 секунд', () => {
    toasts.push({ message: 'Сохранено' });
    expect(toasts.items).toHaveLength(1);
    vi.advanceTimersByTime(5000);
    expect(toasts.items).toHaveLength(0);
  });

  it('действие «Отменить» вызывает колбэк и закрывает тост', () => {
    const undo = vi.fn();
    const id = toasts.push({ message: 'Удалено', action: { label: 'Отменить', run: undo } });
    toasts.runAction(id);
    expect(undo).toHaveBeenCalledOnce();
    expect(toasts.items).toHaveLength(0);
  });

  it('ошибка не пропадает сама', () => {
    toasts.push({ message: 'Не удалось сохранить', kind: 'error', code: 'DB_BUSY' });
    vi.advanceTimersByTime(60_000);
    expect(toasts.items).toHaveLength(1);
  });
});

describe('блокировка', () => {
  it('очищает тосты, чтобы текст и действия не пережили сессию', () => {
    toasts.push({ message: 'Трата удалена', action: { label: 'Отменить', run: () => {} } });
    toasts.push({ message: 'Ошибка', kind: 'error' });
    session.lockedFromBackend();
    expect(toasts.items).toHaveLength(0);
  });
});

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api/call';
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

  it('тост с тем же ключом заменяет предыдущий', () => {
    toasts.push({ message: 'Статус: План', key: 'status' });
    toasts.push({ message: 'Статус: Долг', key: 'status' });
    toasts.push({ message: 'Другое' });
    expect(toasts.items.map((t) => t.message)).toEqual(['Статус: Долг', 'Другое']);
    vi.advanceTimersByTime(5000);
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

describe('fail', () => {
  it('показывает человеческий текст и прячет код журнала в подробности', () => {
    const e = new ApiError({ code: 'Internal', correlationId: 'c-42' });
    toasts.fail(e);
    const [toast] = toasts.items;
    expect(toast?.message).not.toContain('c-42');
    expect(toast?.code).toBe('Код для журнала: c-42');
    expect(toast?.kind).toBe('error');
  });

  it('«Повторить» появляется, только когда есть что повторять', () => {
    const e = new ApiError({ code: 'Conflict', messageKey: 'errors.db.busy' });
    toasts.fail(e);
    expect(toasts.items[0]?.message).toBe(
      'База занята другой операцией. Повторите через пару секунд.'
    );
    expect(toasts.items[0]?.action).toBeUndefined();
    let retried = false;
    const id = toasts.fail(e, () => {
      retried = true;
    });
    toasts.runAction(id);
    expect(retried).toBe(true);
  });
});

describe('время жизни и стек', () => {
  it('обычный тост живёт 5 с, тост с действием — дольше', () => {
    toasts.push({ message: 'Сохранено' });
    toasts.push({ message: 'Трата удалена', action: { label: 'Отменить', run: () => {} } });
    vi.advanceTimersByTime(5000);
    expect(toasts.items.map((t) => t.message)).toEqual(['Трата удалена']);
    vi.advanceTimersByTime(4000);
    expect(toasts.items).toHaveLength(0);
  });

  it('под курсором таймер стоит и начинается заново после ухода', () => {
    const id = toasts.push({ message: 'Сохранено' });
    vi.advanceTimersByTime(4000);
    toasts.pause(id);
    vi.advanceTimersByTime(60_000);
    expect(toasts.items).toHaveLength(1);
    toasts.resume(id);
    vi.advanceTimersByTime(4999);
    expect(toasts.items).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(toasts.items).toHaveLength(0);
  });

  it('в стеке не больше трёх: самый старый уходит первым', () => {
    for (const m of ['1', '2', '3', '4']) toasts.push({ message: m });
    expect(toasts.items.map((t) => t.message)).toEqual(['2', '3', '4']);
  });

  it('смена раздела убирает всё, кроме ошибок', () => {
    toasts.push({ message: 'Статус: План' });
    toasts.push({ message: 'Не удалось', kind: 'error' });
    toasts.dismissOnNavigate();
    expect(toasts.items.map((t) => t.message)).toEqual(['Не удалось']);
  });

  it('вариант success не отличается по времени жизни от info', () => {
    toasts.push({ message: 'Сохранено', kind: 'success' });
    vi.advanceTimersByTime(5000);
    expect(toasts.items).toHaveLength(0);
  });
});

describe('вытеснение', () => {
  it('ошибка не вытесняется обычными тостами, пока их можно вытеснить', () => {
    toasts.push({ message: 'Сбой', kind: 'error' });
    for (const m of ['1', '2', '3']) toasts.push({ message: m });
    expect(toasts.items.map((t) => t.message)).toEqual(['Сбой', '2', '3']);
  });
});

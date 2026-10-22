import { errorDetail, errorText, isRetryable } from '$lib/i18n/errors';
import { onLock } from './session.svelte';

export interface ToastInput {
  message: string;
  kind?: 'info' | 'success' | 'error';
  /** Технические подробности для журнала: прячутся под «Подробности», в основной текст не попадают. */
  code?: string;
  action?: { label: string; run: () => void };
  /** Новый тост с тем же ключом заменяет прежний: серия одинаковых событий не копит стопку. */
  key?: string;
}

export interface ToastItem extends ToastInput {
  id: number;
  /** Сколько тост живёт, мс; `null` — до закрытия пользователем (ошибки). Нужно полосе таймера. */
  ttl: number | null;
  /** Растёт, когда таймер запускается заново (курсор ушёл): полоса начинается с начала. */
  rev: number;
  kind: 'info' | 'success' | 'error';
}

/** Обычный тост; тост с «Отменить» живёт дольше: успеть нажать. Ошибки закрывает пользователь. */
const LIFETIME_MS = 5000;
const ACTION_LIFETIME_MS = 9000;
/** В стеке не больше трёх: четвёртый вытесняет самый старый. */
const MAX_VISIBLE = 3;

class ToastStore {
  items = $state.raw<ToastItem[]>([]);
  #next = 1;
  #timers = new Map<number, ReturnType<typeof setTimeout>>();

  push(input: ToastInput): number {
    if (input.key !== undefined) {
      for (const old of this.items.filter((t) => t.key === input.key)) this.dismiss(old.id);
    }
    const item: ToastItem = {
      ...input,
      id: this.#next++,
      kind: input.kind ?? 'info',
      ttl: null,
      rev: 0
    };
    item.ttl = this.#lifetime(item);
    this.items = [...this.items, item];
    while (this.items.length > MAX_VISIBLE) {
      // вытесняется самый старый тост, но не ошибка: она живёт до закрытия и без неё сбой можно не заметить
      const oldest = this.items.find((t) => t.kind !== 'error') ?? this.items[0];
      if (oldest === undefined) break;
      this.dismiss(oldest.id);
    }
    this.#arm(item);
    return item.id;
  }

  #lifetime(item: ToastItem): number | null {
    // Ошибку закрывает пользователь: пропавшую сама ошибку можно не заметить.
    if (item.kind === 'error') return null;
    return item.action === undefined ? LIFETIME_MS : ACTION_LIFETIME_MS;
  }

  #arm(item: ToastItem): void {
    clearTimeout(this.#timers.get(item.id));
    const ms = this.#lifetime(item);
    if (ms === null) return;
    this.#timers.set(
      item.id,
      setTimeout(() => {
        this.dismiss(item.id);
      }, ms)
    );
  }

  /** Курсор или фокус на тосте: таймер стоит. */
  pause(id: number): void {
    clearTimeout(this.#timers.get(id));
    this.#timers.delete(id);
  }

  /** Курсор ушёл: время жизни отсчитывается заново. */
  resume(id: number): void {
    const item = this.items.find((t) => t.id === id);
    if (item === undefined) return;
    const next = { ...item, rev: item.rev + 1 };
    this.items = this.items.map((t) => (t.id === id ? next : t));
    this.#arm(next);
  }

  /** Смена раздела: тосты прежнего экрана не переезжают на новый, ошибки остаются до закрытия. */
  dismissOnNavigate(): void {
    for (const item of this.items.filter((t) => t.kind !== 'error')) this.dismiss(item.id);
  }

  /** Тост ошибки по пойманному исключению: человеческий текст, код в «Подробностях», «Повторить» — если есть что повторять. */
  fail(e: unknown, retry?: () => void): number {
    const code = errorDetail(e);
    // «Повторить» только там, где повтор может помочь: занятая база и внутренние сбои, а не ошибки ввода
    const canRetry = retry !== undefined && isRetryable(e);
    return this.push({
      kind: 'error',
      message: errorText(e),
      ...(code === null ? {} : { code }),
      ...(canRetry ? { action: { label: 'Повторить', run: retry } } : {})
    });
  }

  runAction(id: number): void {
    this.items.find((item) => item.id === id)?.action?.run();
    this.dismiss(id);
  }

  dismiss(id: number): void {
    clearTimeout(this.#timers.get(id));
    this.#timers.delete(id);
    this.items = this.items.filter((item) => item.id !== id);
  }

  clear(): void {
    for (const timer of this.#timers.values()) clearTimeout(timer);
    this.#timers.clear();
    this.items = [];
  }
}

export const toasts = new ToastStore();

// Текст и действия тостов могут нести данные бюджета: при блокировке не остаётся ничего.
onLock(() => {
  toasts.clear();
});

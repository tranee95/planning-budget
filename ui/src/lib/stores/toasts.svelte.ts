import { onLock } from './session.svelte';

export interface ToastInput {
  message: string;
  kind?: 'info' | 'error';
  /** Код ошибки для логов: показывается мелким шрифтом. */
  code?: string;
  action?: { label: string; run: () => void };
}

export interface ToastItem extends ToastInput {
  id: number;
  kind: 'info' | 'error';
}

const LIFETIME_MS = 5000;

class ToastStore {
  items = $state.raw<ToastItem[]>([]);
  #next = 1;
  #timers = new Map<number, ReturnType<typeof setTimeout>>();

  push(input: ToastInput): number {
    const item: ToastItem = { ...input, id: this.#next++, kind: input.kind ?? 'info' };
    this.items = [...this.items, item];
    // Ошибку закрывает пользователь: пропавшую сама ошибку можно не заметить.
    if (item.kind === 'info') {
      this.#timers.set(
        item.id,
        setTimeout(() => {
          this.dismiss(item.id);
        }, LIFETIME_MS)
      );
    }
    return item.id;
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

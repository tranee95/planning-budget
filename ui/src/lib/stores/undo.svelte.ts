import { errorText } from '$lib/i18n/errors';
import { onLock } from './session.svelte';
import { toasts } from './toasts.svelte';

export interface UndoEntry {
  /** Короткое описание для тоста: «Трата удалена». */
  label: string;
  undo: () => Promise<void> | void;
  redo: () => Promise<void> | void;
}

const MAX_DEPTH = 50;

/** Стек действий сессии (⌘Z / ⇧⌘Z). Живёт в памяти и очищается при блокировке. */
class UndoStack {
  #done = $state.raw<UndoEntry[]>([]);
  #undone = $state.raw<UndoEntry[]>([]);
  #busy = false;

  canUndo = $derived(this.#done.length > 0);
  canRedo = $derived(this.#undone.length > 0);
  get depth(): number {
    return this.#done.length;
  }

  record(entry: UndoEntry): void {
    this.#done = [...this.#done, entry].slice(-MAX_DEPTH);
    this.#undone = [];
  }

  /** Без аргумента отменяет последнее действие; с аргументом — именно эту запись (тост «Отменить»). */
  async undo(target?: UndoEntry): Promise<void> {
    const entry = target ?? this.#done.at(-1);
    if (!entry || !this.#done.includes(entry) || this.#busy) return;
    if (!(await this.#run(entry.undo))) return;
    this.#done = this.#done.filter((e) => e !== entry);
    this.#undone = [...this.#undone, entry];
    toasts.push({ message: `Отменено: ${entry.label}` });
  }

  async redo(): Promise<void> {
    const entry = this.#undone.at(-1);
    if (!entry || this.#busy) return;
    if (!(await this.#run(entry.redo))) return;
    this.#undone = this.#undone.slice(0, -1);
    this.#done = [...this.#done, entry];
    toasts.push({ message: `Повторено: ${entry.label}` });
  }

  clear(): void {
    this.#done = [];
    this.#undone = [];
  }

  /** При ошибке запись остаётся на месте: пользователь может повторить. */
  async #run(action: () => Promise<void> | void): Promise<boolean> {
    this.#busy = true;
    try {
      await action();
      return true;
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    } finally {
      this.#busy = false;
    }
  }
}

export const undoStack = new UndoStack();

// Замыкания записей держат id и значения бюджета.
onLock(() => {
  undoStack.clear();
});

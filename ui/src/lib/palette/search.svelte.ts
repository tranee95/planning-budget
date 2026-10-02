import type { SearchResultDto } from '$lib/api/bindings';
import { onLock } from '$lib/stores/session.svelte';

export type SearchFn = (query: string, requestId: number) => Promise<SearchResultDto>;

/** Дебаунс ввода. */
export const SEARCH_DEBOUNCE_MS = 70;

const RECENT_LIMIT = 5;

/** Последние запросы палитры: только в памяти сессии, на диск не попадают. */
class RecentQueries {
  items = $state<string[]>([]);

  remember(query: string): void {
    const text = query.trim();
    if (text === '') return;
    this.items = [text, ...this.items.filter((q) => q !== text)].slice(0, RECENT_LIMIT);
  }

  reset(): void {
    this.items = [];
  }
}

export const recentQueries = new RecentQueries();
onLock(() => {
  recentQueries.reset();
});

/**
 * Поиск палитры: дебаунс, нумерация запросов и отбрасывание устаревших ответов.
 * `result.query` — строка, для которой посчитаны позиции токенов: пока вводят дальше,
 * чипы строятся по ней, а не по текущему тексту.
 */
export class PaletteSearch {
  query = $state('');
  result = $state<{ query: string; data: SearchResultDto } | null>(null);
  pending = $state(false);

  #run: SearchFn;
  #delay: number;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #counter = 0;

  constructor(run: SearchFn, delay: number = SEARCH_DEBOUNCE_MS) {
    this.#run = run;
    this.#delay = delay;
  }

  set(query: string): void {
    this.query = query;
    clearTimeout(this.#timer);
    this.#counter += 1;
    if (query.trim() === '') {
      this.result = null;
      this.pending = false;
      return;
    }
    this.pending = true;
    this.#timer = setTimeout(() => {
      void this.#fire(query);
    }, this.#delay);
  }

  async #fire(query: string): Promise<void> {
    const id = this.#counter;
    try {
      const data = await this.#run(query, id);
      if (id !== this.#counter) return;
      this.result = { query, data };
    } catch {
      // Ошибка поиска не должна ломать палитру: команды остаются доступны.
      if (id !== this.#counter) return;
      this.result = null;
    }
    this.pending = false;
  }

  dispose(): void {
    clearTimeout(this.#timer);
    this.#counter += 1;
  }
}

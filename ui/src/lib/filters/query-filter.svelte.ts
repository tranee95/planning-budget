import type { QueryHintSpanDto, TokenSpanDto } from '$lib/api/bindings';

/** Дебаунс ввода. */
export const FILTER_DEBOUNCE_MS = 70;

/** Разбор и итоги последнего применённого запроса: по ним рисуются чипы и строка «Найдено». */
export interface FilterMeta {
  /** Строка, для которой посчитаны `spans` и `hints`. */
  query: string;
  spans: TokenSpanDto[];
  hints: QueryHintSpanDto[];
  total: number;
  sum: number;
  truncated: boolean;
}

/**
 * Запрос панели фильтров: поле ввода (`text`) и применённый запрос (`applied`) с дебаунсом.
 * Данные по запросу грузит ViewModel экрана и кладёт итоги в `meta`.
 */
export class QueryFilter {
  text = $state('');
  applied = $state('');
  meta = $state.raw<FilterMeta | null>(null);

  active = $derived(this.applied !== '');

  #delay: number;
  #timer: ReturnType<typeof setTimeout> | undefined;

  constructor(delay: number = FILTER_DEBOUNCE_MS) {
    this.#delay = delay;
  }

  /** Ввод в поле: запрос применяется после паузы, пустой — сразу. */
  type(text: string): void {
    this.text = text;
    clearTimeout(this.#timer);
    if (text.trim() === '') {
      this.applied = '';
      return;
    }
    this.#timer = setTimeout(() => {
      this.applied = text.trim();
    }, this.#delay);
  }

  /** Подставляет запрос целиком и применяет сразу: сохранённый фильтр, крестик на чипе, ссылка из палитры. */
  set(text: string): void {
    clearTimeout(this.#timer);
    this.text = text;
    this.applied = text.trim();
  }

  clear(): void {
    this.set('');
    this.meta = null;
  }

  dispose(): void {
    clearTimeout(this.#timer);
  }
}

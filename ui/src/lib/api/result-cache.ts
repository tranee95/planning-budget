import { events } from '$lib/api/bindings';
import { onLock } from '$lib/stores/session.svelte';

/**
 * Кеш готовых результатов расчётов Rust (данные графиков). Ключ — запрос целиком; значение
 * верно, пока данные не менялись, поэтому любое `data-changed` и блокировка хранилища очищают
 * всё. Ответ, начатый до изменения, в кеш не попадает (`epoch`): иначе устаревший результат
 * пережил бы очистку.
 */
export class ResultCache {
  #entries = new Map<string, unknown>();
  #epoch = 0;
  readonly #limit: number;

  constructor(limit = 200) {
    this.#limit = limit;
  }

  /** Номер поколения: запоминается перед запросом и передаётся в `set`. */
  get epoch(): number {
    return this.#epoch;
  }

  get(key: string): unknown {
    return this.#entries.get(key);
  }

  set(key: string, value: unknown, epoch: number): void {
    if (epoch !== this.#epoch || this.#disabled) return;
    this.#entries.delete(key);
    this.#entries.set(key, value);
    if (this.#entries.size > this.#limit) {
      const oldest = this.#entries.keys().next();
      if (!oldest.done) this.#entries.delete(oldest.value);
    }
  }

  clear(): void {
    this.#entries.clear();
    this.#epoch += 1;
  }

  /** Кеш отключён до восстановления подписки: ничего не запоминает. */
  #disabled = false;

  disable(): void {
    this.#disabled = true;
    this.clear();
  }

  enable(): void {
    this.#disabled = false;
  }
}

export const resultCache = new ResultCache();

let watching: Promise<() => void> | undefined;
let lockRegistered = false;

/**
 * Очистка кеша по любому изменению данных. Подписка сырая, без склейки по времени:
 * кеш не должен успеть отдать устаревшее в окне коалесценции. Блокировка очищает кеш тоже.
 */
export function watchResultCache(): void {
  if (watching !== undefined) return;
  resultCache.enable();
  if (!lockRegistered) {
    lockRegistered = true;
    onLock(() => {
      resultCache.clear();
    });
  }
  watching = events.dataChanged.listen(() => {
    resultCache.clear();
  });
  // Без подписки кеш не узнает об изменениях: при сбое он отключается (очищается и больше не
  // заполняется), а подписка пробуется заново при следующем вызове.
  watching.catch(() => {
    watching = undefined;
    resultCache.disable();
  });
}

/** Текущая дата `ГГГГ-ММ-ДД` по местному времени: расчёты Rust зависят от «сегодня». */
export function dayKey(now: Date = new Date()): string {
  const pad = (n: number): string => String(n).padStart(2, '0');
  return `${String(now.getFullYear())}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** Результат расчёта из кеша или из `load`; сбой и ответ, начатый до изменения данных, не кешируются. */
export async function cachedCall<T>(key: string, load: () => Promise<T>): Promise<T> {
  const hit = resultCache.get(key) as T | undefined;
  if (hit !== undefined) return hit;
  const epoch = resultCache.epoch;
  const value = await load();
  resultCache.set(key, value, epoch);
  return value;
}

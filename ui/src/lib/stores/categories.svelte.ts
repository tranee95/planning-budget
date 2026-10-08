import { type CategoryDto, type ChangeScope } from '$lib/api/bindings';
import { onDataChanged } from '$lib/api/data-events';
import { categoriesApi } from '$lib/api/data';
import { onLock } from './session.svelte';

/** Правки этих областей меняют список категорий или их порядок. */
export const affectsCategories = (scope: ChangeScope): boolean =>
  scope === 'categories' || scope === 'all';

/** Активные категории: нужны «Расходам», «Доходам», быстрому добавлению и поиску. */
class CategoriesStore {
  items = $state.raw<CategoryDto[]>([]);
  loaded = $state(false);

  /** Порядок `items` задан Rust по `sortOrder`. */
  byId = $derived(new Map(this.items.map((c) => [c.id, c])));

  async load(): Promise<void> {
    this.items = await categoriesApi.list(false);
    this.loaded = true;
  }

  /** Грузит один раз; повторные вызовы возвращают уже загруженное. */
  async ensure(): Promise<void> {
    if (!this.loaded) await this.load();
  }

  /** Перечитывает список, когда категории изменились (редактор, импорт). Возвращает cleanup. */
  watch(): () => void {
    return onDataChanged(({ scope }) => {
      if (this.loaded && affectsCategories(scope)) void this.#refresh();
    });
  }

  #refreshing = false;
  #requested = 0;

  /** Одна загрузка за раз: события, пришедшие во время неё, дают ровно одну повторную. */
  async #refresh(): Promise<void> {
    this.#requested++;
    if (this.#refreshing) return;
    this.#refreshing = true;
    try {
      let served = -1;
      while (served !== this.#requested) {
        served = this.#requested;
        await this.load();
      }
    } finally {
      this.#refreshing = false;
    }
  }

  reset(): void {
    this.items = [];
    this.loaded = false;
  }
}

export const categories = new CategoriesStore();

// Названия категорий — данные бюджета: при блокировке стор очищается.
onLock(() => {
  categories.reset();
});

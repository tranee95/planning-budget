import { events, type CategoryDto, type ChangeScope } from '$lib/api/bindings';
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
    const off = events.dataChanged.listen(({ payload }) => {
      if (this.loaded && affectsCategories(payload.scope)) void this.load();
    });
    return () => {
      void off.then((stop) => {
        stop();
      });
    };
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

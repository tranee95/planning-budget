import type { SavedFilterDto } from '$lib/api/bindings';
import { filtersApi } from '$lib/api/data';
import { onLock } from './session.svelte';

/** Сохранённые фильтры (`saved_filters`): панель фильтров и пустая палитра. */
class SavedFiltersStore {
  items = $state.raw<SavedFilterDto[]>([]);
  loaded = $state(false);

  async load(): Promise<void> {
    this.items = await filtersApi.list();
    this.loaded = true;
  }

  async ensure(): Promise<void> {
    if (!this.loaded) await this.load();
  }

  /** Фильтр с тем же именем получает новый запрос. */
  async save(name: string, query: string): Promise<SavedFilterDto> {
    const saved = await filtersApi.save(name, query);
    await this.load();
    return saved;
  }

  async remove(id: number): Promise<void> {
    await filtersApi.remove(id);
    this.items = this.items.filter((f) => f.id !== id);
  }

  reset(): void {
    this.items = [];
    this.loaded = false;
  }
}

export const savedFilters = new SavedFiltersStore();

// Запросы могут содержать названия трат: при блокировке стор очищается.
onLock(() => {
  savedFilters.reset();
});

import type { FilterScreenDto, SavedFilterDto } from '$lib/api/bindings';
import { filtersApi } from '$lib/api/data';
import { onLock } from './session.svelte';
import { undoStack } from './undo.svelte';

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

  /** Фильтр с тем же именем получает новый запрос. Действие попадает в стек отмены. */
  async save(name: string, query: string, screen: FilterScreenDto): Promise<SavedFilterDto> {
    const key = name.trim().toLowerCase();
    const previous = this.items.find((f) => f.screen === screen && f.name.toLowerCase() === key);
    let current = await filtersApi.save(name, query, screen);
    await this.load();
    const entry = { ...current };
    undoStack.record({
      label: 'сохранение фильтра',
      undo: async () => {
        if (previous)
          current = await filtersApi.save(previous.name, previous.query, previous.screen);
        else await filtersApi.remove(current.id);
        await this.load();
      },
      redo: async () => {
        current = await filtersApi.save(entry.name, entry.query, entry.screen);
        await this.load();
      }
    });
    return current;
  }

  /** Удалённый фильтр возвращается при отмене (с новым id). */
  async remove(id: number): Promise<void> {
    const removed = this.items.find((f) => f.id === id);
    await filtersApi.remove(id);
    this.items = this.items.filter((f) => f.id !== id);
    if (!removed) return;
    let currentId = id;
    undoStack.record({
      label: 'удаление фильтра',
      undo: async () => {
        currentId = (await filtersApi.save(removed.name, removed.query, removed.screen)).id;
        await this.load();
      },
      redo: async () => {
        await filtersApi.remove(currentId);
        await this.load();
      }
    });
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

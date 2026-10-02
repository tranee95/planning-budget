import type { TagDto } from '$lib/api/bindings';
import { tagsApi } from '$lib/api/data';
import { onLock } from './session.svelte';

/** Теги трат: колонка «Теги» таблицы и выбор тегов строки. */
class TagsStore {
  items = $state.raw<TagDto[]>([]);
  loaded = $state(false);

  byId = $derived(new Map(this.items.map((t) => [t.id, t])));

  async load(): Promise<void> {
    this.items = await tagsApi.list();
    this.loaded = true;
  }

  async ensure(): Promise<void> {
    if (!this.loaded) await this.load();
  }

  /** Создаёт тег и добавляет его в список; имя уже занято — Rust вернёт `Conflict`. */
  async create(name: string): Promise<TagDto> {
    const tag = await tagsApi.create(name);
    this.items = [...this.items, tag].sort((a, b) => a.name.localeCompare(b.name, 'ru'));
    return tag;
  }

  reset(): void {
    this.items = [];
    this.loaded = false;
  }
}

export const tags = new TagsStore();

// Названия тегов — данные бюджета: при блокировке стор очищается.
onLock(() => {
  tags.reset();
});

import type { TagDto } from '$lib/api/bindings';
import { tagsApi } from '$lib/api/data';
import { onLock } from './session.svelte';

const sorted = (items: TagDto[]): TagDto[] =>
  items.sort((a, b) => a.name.localeCompare(b.name, 'ru'));

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
    this.items = sorted([...this.items, tag]);
    return tag;
  }

  /** Переименовывает тег; занятое имя — `Conflict` от Rust. */
  async rename(id: number, name: string): Promise<TagDto> {
    const tag = await tagsApi.rename(id, name);
    this.items = sorted(this.items.map((t) => (t.id === id ? tag : t)));
    return tag;
  }

  /** Удаляет тег и возвращает, у скольких трат он стоял. */
  async remove(id: number): Promise<number> {
    const uses = await tagsApi.remove(id);
    this.items = this.items.filter((t) => t.id !== id);
    return uses;
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

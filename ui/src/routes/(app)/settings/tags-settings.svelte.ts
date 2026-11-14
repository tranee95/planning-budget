import type { TagUsageDto } from '$lib/api/bindings';
import { tagsApi } from '$lib/api/data';
import { errorText } from '$lib/i18n/errors';
import { pluralRu } from '$lib/format';
import { tags } from '$lib/stores/tags.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';

/** «5 трат», «1 трата», «2 траты». */
export function usesText(uses: number): string {
  return `${String(uses)} ${pluralRu(uses, ['трата', 'траты', 'трат'])}`;
}

/** «1 сохранённый запрос», «3 сохранённых запроса» (фильтры и карточки графиков). */
export function refsText(refs: number): string {
  return `${String(refs)} ${pluralRu(refs, ['сохранённый запрос', 'сохранённых запроса', 'сохранённых запросов'])}`;
}

/** Текст подтверждения удаления: сколько трат потеряют тег и какие запросы перестанут его находить. */
export function deleteHint(tag: TagUsageDto): string {
  const parts = [
    tag.uses === 0
      ? 'Тег нигде не используется.'
      : `Тег стоит на ${usesText(tag.uses)}: он будет снят с них.`
  ];
  if (tag.refs > 0) {
    parts.push(
      `${refsText(tag.refs)} (фильтры и графики) называют этот тег: они перестанут находить траты.`
    );
  }
  parts.push('Вернуть тег нельзя.');
  return parts.join(' ');
}

/** Подсказка в окне переименования: запросы с этим тегом получат новое имя. */
export function renameHint(tag: TagUsageDto): string {
  return tag.refs === 0
    ? ''
    : `Имя тега в ${refsText(tag.refs)} (фильтры и графики) будет заменено на новое.`;
}

/** Управление тегами в настройках: список с числом трат, переименование и удаление. */
export class TagsVm {
  loaded = $state(false);
  error = $state<string | null>(null);

  items = $state.raw<TagUsageDto[]>([]);

  async load(): Promise<void> {
    try {
      this.items = await tagsApi.usage();
      this.loaded = true;
      this.error = null;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** Переименовывает тег; `false` и `error`, если имя пустое или занято. */
  async rename(id: number, name: string): Promise<boolean> {
    try {
      await tags.rename(id, name);
      await this.load();
      this.error = null;
      toasts.push({ message: `Тег переименован: ${name.trim()}` });
      return true;
    } catch (e) {
      this.error = errorText(e);
      return false;
    }
  }

  /** Удаляет тег и снимает его со всех трат. */
  async remove(tag: TagUsageDto): Promise<boolean> {
    try {
      const uses = await tags.remove(tag.id);
      undoStack.forgetTag(tag.id);
      await this.load();
      this.error = null;
      toasts.push({
        message:
          uses === 0
            ? `Тег «${tag.name}» удалён`
            : `Тег «${tag.name}» удалён и снят с ${usesText(uses)}`
      });
      return true;
    } catch (e) {
      this.error = errorText(e);
      return false;
    }
  }

  clearError(): void {
    this.error = null;
  }
}

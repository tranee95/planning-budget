import { SvelteSet } from 'svelte/reactivity';
import type { CategoryKindDto, DefaultCategoryDto } from '$lib/api/bindings';
import { categoriesApi } from '$lib/api/data';
import { errorText } from '$lib/i18n/errors';

const KIND_LABEL: Record<CategoryKindDto, string> = {
  mandatory: 'Обязательные',
  wants: 'Желания',
  loans: 'Займы',
  savings: 'Сбережения'
};
const KIND_ORDER: readonly CategoryKindDto[] = ['mandatory', 'wants', 'loans', 'savings'];

export interface CategoryGroup {
  kind: CategoryKindDto;
  label: string;
  items: readonly DefaultCategoryDto[];
}

/**
 * Выбор стартовых категорий в знакомстве. Шаг нужен только пока в хранилище нет категорий:
 * «Показать знакомство снова» на рабочем бюджете ничего не создаёт.
 */
export class CategoryPickerVm {
  /** Список категорий проверен: до этого окно не показывается, чтобы число шагов не прыгало. */
  ready = $state(false);
  /** Пусто, если категории уже есть. */
  options = $state.raw<readonly DefaultCategoryDto[]>([]);
  readonly chosen = new SvelteSet<string>();
  busy = $state(false);
  /** Чтение списка не удалось: неизвестно, нужен ли шаг. */
  failed = false;
  error = $state<string | null>(null);

  needed = $derived(this.options.length > 0);
  groups = $derived<readonly CategoryGroup[]>(
    KIND_ORDER.map((kind) => ({
      kind,
      label: KIND_LABEL[kind],
      items: this.options.filter((c) => c.kind === kind)
    })).filter((group) => group.items.length > 0)
  );

  async load(): Promise<void> {
    try {
      // Архивные тоже считаются: бэкенд сидирует только полностью пустую таблицу.
      if ((await categoriesApi.list(true)).length === 0) {
        const options = await categoriesApi.defaults();
        this.options = options;
        // По умолчанию отмечено всё: пропуск знакомства даёт тот же набор, что и раньше.
        this.setAll(true);
      }
      this.failed = false;
    } catch {
      // Окно не блокируем: при выходе чтение повторится один раз (`apply`).
      this.options = [];
      this.failed = true;
    }
    this.ready = true;
  }

  isChosen(name: string): boolean {
    return this.chosen.has(name);
  }

  toggle(name: string): void {
    if (!this.chosen.delete(name)) this.chosen.add(name);
  }

  /** Отметить всё или оставить только обязательные. */
  setAll(on: boolean): void {
    this.chosen.clear();
    for (const c of this.options) if (on || c.required) this.chosen.add(c.name);
  }

  /** Создаёт отмеченные категории; `false`, если не вышло или выход уже идёт (текст в `error`). */
  async apply(): Promise<boolean> {
    // Флаг ставится до первого await: повторный выход во время чтения не запускает второй apply.
    if (this.busy) return false;
    this.busy = true;
    this.error = null;
    try {
      if (this.failed) await this.load();
      // Нельзя пометить знакомство пройденным, не узнав, нужен ли выбор: иначе сейф останется без категорий.
      if (this.failed) {
        this.error = 'Не удалось проверить категории. Повторите попытку.';
        return false;
      }
      if (!this.needed) return true;
      await categoriesApi.seed([...this.chosen]);
      this.options = [];
      return true;
    } catch (e) {
      this.error = errorText(e);
      return false;
    } finally {
      this.busy = false;
    }
  }
}

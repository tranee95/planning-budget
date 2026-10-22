import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import {
  type BudgetBalanceDto,
  type CategoryDto,
  type CategoryKindDto,
  type CategoryYearRowDto,
  type LimitRowDto,
  type SettingsDto
} from '$lib/api/bindings';
import { LoadStamp, onDataChanged } from '$lib/api/data-events';
import { categoriesApi, settingsApi, summaryApi } from '$lib/api/data';
import { CATEGORY_COLORS } from '$lib/category-colors';
import { pluralRu } from '$lib/format';
import { errorText } from '$lib/i18n/errors';
import { toasts } from '$lib/stores/toasts.svelte';

export type CategoryRow = {
  category: CategoryDto;
  /** Лимит в выбранном месяце (для категории сбережений — `null`). */
  limit: number | null;
  avg: number;
  /** Среднее минус лимит: положительное — превышение. */
  overLimit: number | null;
};

export type Draft = {
  name: string;
  color: string;
  kind: CategoryKindDto;
  note: string;
  /** Копейки; `null` — поле пусто. */
  limit: number | null;
};

const emptyDraft = (): Draft => ({
  name: '',
  color: CATEGORY_COLORS[0] ?? '',
  kind: 'wants',
  note: '',
  limit: null
});

/** ViewModel экрана «Категории и лимиты». Суммы и средние приходят из Rust. */
export class CategoriesVm {
  items = $state.raw<CategoryDto[]>([]);
  limits = $state.raw<LimitRowDto[]>([]);
  yearRows = $state.raw<CategoryYearRowDto[]>([]);
  balance = $state.raw<BudgetBalanceDto | null>(null);
  settings = $state.raw<SettingsDto | null>(null);
  showArchived = $state(false);
  /** Архивных категорий (кроме сбережений): «Показать архив» при нуле объясняет, что показывать нечего. */
  archivedCount = $derived(this.items.filter((c) => c.kind !== 'savings' && c.archived).length);
  loading = $state(false);
  error = $state<string | null>(null);

  /** `null` — редактор закрыт; `'new'` — создание. */
  selected = $state<number | 'new' | null>(null);
  draft = $state<Draft>(emptyDraft());
  saving = $state(false);
  fieldError = $state<string | null>(null);

  #month = '';
  #req = 0;
  #stamp = new LoadStamp();
  /** Идёт серия собственных команд: события от них не нужны, экран перечитается сам. */
  #writes = 0;

  get month(): string {
    return this.#month;
  }

  rows = $derived.by<CategoryRow[]>(() => {
    const limitById = new SvelteMap(this.limits.map((l) => [l.categoryId, l]));
    const yearById = new SvelteMap(this.yearRows.map((r) => [r.categoryId, r]));
    return (
      this.items
        // Накопления живут во вкладке «Сбережения».
        .filter((c) => c.kind !== 'savings' && (this.showArchived || !c.archived))
        .map((category) => {
          const year = yearById.get(category.id);
          const row = limitById.get(category.id);
          return {
            category,
            limit: row?.limit ?? null,
            avg: year?.avg ?? 0,
            overLimit: year?.avgMinusLimit ?? null
          };
        })
    );
  });

  subtitle = $derived.by(() => {
    const n = this.items.filter((c) => c.kind !== 'savings' && !c.archived).length;
    return `${String(n)} ${pluralRu(n, ['категория', 'категории', 'категорий'])} · перетаскивайте, чтобы изменить порядок`;
  });

  current = $derived(
    typeof this.selected === 'number'
      ? (this.items.find((c) => c.id === this.selected) ?? null)
      : null
  );

  async load(month: string): Promise<void> {
    this.#stamp.mark();
    const req = ++this.#req;
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const [items, overview, year, settings] = await Promise.all([
        categoriesApi.list(true),
        summaryApi.month(month),
        summaryApi.year(Number(month.slice(0, 4))),
        settingsApi.get()
      ]);
      if (req !== this.#req) return;
      this.items = items;
      this.limits = overview.limits;
      this.yearRows = year.categories;
      this.balance = year.balance;
      this.settings = settings;
      if (typeof this.selected === 'number' && !items.some((c) => c.id === this.selected)) {
        this.selected = null;
      }
    } catch (e) {
      if (req === this.#req) this.error = errorText(e);
    } finally {
      if (req === this.#req) this.loading = false;
    }
  }

  select(id: number): void {
    const row = this.rows.find((r) => r.category.id === id);
    if (!row) return;
    this.selected = id;
    this.fieldError = null;
    this.draft = {
      name: row.category.name,
      color: row.category.color,
      kind: row.category.kind,
      note: row.category.note ?? '',
      limit: row.limit
    };
  }

  startCreate(): void {
    this.selected = 'new';
    this.fieldError = null;
    this.draft = emptyDraft();
  }

  closeEditor(): void {
    this.selected = null;
  }

  /** Сохраняет черновик: поля категории, затем лимит «с начала месяца экрана». */
  async save(): Promise<boolean> {
    const d = this.draft;
    const name = d.name.trim();
    if (name === '') {
      this.fieldError = 'Название не должно быть пустым';
      return false;
    }
    this.fieldError = null;
    this.saving = true;
    this.#writes++;
    try {
      const note = d.note.trim() === '' ? null : d.note.trim();
      let id: number;
      if (this.selected === 'new') {
        id = (await categoriesApi.create({ name, kind: d.kind, color: d.color, note })).id;
        // Повторное «Сохранить» после сбоя дальше обновит эту категорию, а не создаст вторую.
        this.selected = id;
      } else if (this.current) {
        id = this.current.id;
        await categoriesApi.update(id, { name, color: d.color, note });
      } else {
        return false;
      }
      const row = this.rows.find((r) => r.category.id === id);
      if (d.limit !== null && d.limit > 0 && d.limit !== row?.limit) {
        await categoriesApi.setLimit(id, this.#month, d.limit);
      }
      // Пустое поле у категории с лимитом: лимита нет с начала месяца экрана.
      if (d.limit === null && row?.limit != null) {
        await categoriesApi.unsetLimit(id, this.#month);
      }
      await this.load(this.#month);
      // Панель редактора закрывается после сохранения: под ней остаётся список с обновлённой строкой.
      this.selected = null;
      toasts.push({ message: `Сохранено: ${name}`, kind: 'success' });
      return true;
    } catch (e) {
      this.fieldError = errorText(e);
      // Часть команд могла пройти (категория создана, лимит нет): события во время серии
      // отброшены, поэтому экран перечитывается сам.
      void this.load(this.#month);
      return false;
    } finally {
      this.#writes--;
      this.saving = false;
    }
  }

  /** Категория, которую пользователь собрался отправить в архив: ждёт подтверждения. */
  archiveAsk = $state<{ id: number; name: string } | null>(null);

  /** Запрос на архивацию: сама категория меняется только после `confirmArchive`. */
  askArchive(id: number): void {
    const category = this.items.find((c) => c.id === id);
    if (category === undefined || category.archived) return;
    this.archiveAsk = { id, name: category.name };
  }

  cancelArchive(): void {
    this.archiveAsk = null;
  }

  async confirmArchive(): Promise<void> {
    const ask = this.archiveAsk;
    if (ask === null) return;
    this.archiveAsk = null;
    await this.setArchived(ask.id, true);
  }

  async setArchived(id: number, archived: boolean): Promise<void> {
    this.#writes++;
    try {
      await (archived ? categoriesApi.archive(id) : categoriesApi.unarchive(id));
      await this.load(this.#month);
      if (archived) this.selected = null;
      toasts.push({ message: archived ? 'Категория в архиве' : 'Категория возвращена' });
    } catch (e) {
      toasts.fail(e, () => {
        void this.setArchived(id, archived);
      });
    } finally {
      this.#writes--;
    }
  }

  /** Переносит `id` на место `targetId`; порядок сохраняется в Rust. */
  async move(id: number, targetId: number): Promise<void> {
    if (id === targetId) return;
    const ids = this.items.map((c) => c.id);
    const from = ids.indexOf(id);
    const to = ids.indexOf(targetId);
    if (from < 0 || to < 0) return;
    ids.splice(to, 0, ...ids.splice(from, 1));
    await this.#reorder(ids);
  }

  async #reorder(ids: number[]): Promise<void> {
    const prev = this.items;
    const byId = new SvelteMap(prev.map((c) => [c.id, c]));
    this.items = ids.flatMap((id) => {
      const c = byId.get(id);
      return c ? [c] : [];
    });
    this.#writes++;
    try {
      this.items = await categoriesApi.reorder(ids);
      // Ответ команды уже содержит новый порядок: событие о нём перечитывать не нужно.
      this.#stamp.mark();
    } catch (e) {
      this.items = prev;
      toasts.fail(e);
    } finally {
      this.#writes--;
    }
  }

  connect(): () => void {
    return onDataChanged((change) => {
      if (this.#writes > 0 || this.#stamp.isFresh(change)) return;
      if (this.#month !== '') void this.load(this.#month);
    });
  }
}

const KEY = Symbol('CategoriesVm');
export const setCategoriesVm = (vm: CategoriesVm) => setContext(KEY, vm);
export const getCategoriesVm = () => getContext<CategoriesVm>(KEY);

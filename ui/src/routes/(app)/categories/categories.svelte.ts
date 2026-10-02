import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import {
  events,
  type BudgetBalanceDto,
  type CategoryDto,
  type CategoryKindDto,
  type CategoryYearRowDto,
  type LimitRowDto,
  type SavingsRateDto,
  type SettingsDto
} from '$lib/api/bindings';
import { categoriesApi, settingsApi, summaryApi } from '$lib/api/data';
import { CATEGORY_COLORS, parsePercentBp } from '$lib/category-colors';
import { errorText } from '$lib/i18n/errors';
import { toasts } from '$lib/stores/toasts.svelte';

export type CategoryRow = {
  category: CategoryDto;
  /** Лимит в выбранном месяце (для категории сбережений — `null`). */
  limit: number | null;
  /** Процент плана сбережений в выбранном месяце, б. п. */
  rateBp: number | null;
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
  /** Процент текстом, как вводит пользователь. */
  rate: string;
  /** Процент только на выбранный месяц, текстом. */
  monthRate: string;
};

const emptyDraft = (): Draft => ({
  name: '',
  color: CATEGORY_COLORS[0] ?? '',
  kind: 'wants',
  note: '',
  limit: null,
  rate: '',
  monthRate: ''
});

/** ViewModel экрана «Категории и лимиты». Суммы и средние приходят из Rust. */
export class CategoriesVm {
  items = $state.raw<CategoryDto[]>([]);
  limits = $state.raw<LimitRowDto[]>([]);
  yearRows = $state.raw<CategoryYearRowDto[]>([]);
  rates = $state.raw<SavingsRateDto[]>([]);
  balance = $state.raw<BudgetBalanceDto | null>(null);
  /** Процент плана сбережений месяца с учётом переопределения, б. п. (считает Rust). */
  monthPlanBp = $state<number | null>(null);
  settings = $state.raw<SettingsDto | null>(null);
  showArchived = $state(false);
  loading = $state(false);
  error = $state<string | null>(null);

  /** `null` — редактор закрыт; `'new'` — создание. */
  selected = $state<number | 'new' | null>(null);
  draft = $state<Draft>(emptyDraft());
  saving = $state(false);
  fieldError = $state<string | null>(null);

  #month = '';
  #req = 0;

  get month(): string {
    return this.#month;
  }

  rows = $derived.by<CategoryRow[]>(() => {
    const limitById = new SvelteMap(this.limits.map((l) => [l.categoryId, l.limit]));
    const yearById = new SvelteMap(this.yearRows.map((r) => [r.categoryId, r]));
    // Процент действует с последней даты не позже начала месяца.
    const rateById = new SvelteMap<number, SavingsRateDto>();
    for (const rate of this.rates) {
      if (rate.validFrom > this.#month) continue;
      const known = rateById.get(rate.categoryId);
      if (!known || known.validFrom < rate.validFrom) rateById.set(rate.categoryId, rate);
    }
    return this.items
      .filter((c) => this.showArchived || !c.archived)
      .map((category) => {
        const year = yearById.get(category.id);
        return {
          category,
          limit: limitById.get(category.id) ?? null,
          rateBp: rateById.get(category.id)?.rateBp ?? null,
          avg: year?.avg ?? 0,
          overLimit: year?.avgMinusLimit ?? null
        };
      });
  });

  subtitle = $derived.by(() => {
    const n = this.items.filter((c) => !c.archived).length;
    return `${String(n)} ${pluralCategories(n)} · перетаскивайте, чтобы изменить порядок`;
  });

  /** Процент плана расходится с нормой из настроек: пользователю нужно это объяснить. */
  rateMismatch = $derived(
    this.settings !== null &&
      this.monthPlanBp !== null &&
      this.monthPlanBp > 0 &&
      this.monthPlanBp !== this.settings.savingsNormBp
  );

  current = $derived(
    typeof this.selected === 'number'
      ? (this.items.find((c) => c.id === this.selected) ?? null)
      : null
  );

  async load(month: string): Promise<void> {
    const req = ++this.#req;
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const [items, overview, year, rates, settings] = await Promise.all([
        categoriesApi.list(true),
        summaryApi.month(month),
        summaryApi.year(Number(month.slice(0, 4))),
        categoriesApi.savingsRates(),
        settingsApi.get()
      ]);
      if (req !== this.#req) return;
      this.items = items;
      this.limits = overview.limits;
      this.yearRows = year.categories;
      this.balance = year.balance;
      this.monthPlanBp = overview.summary.savingsPlanRateBp;
      this.rates = rates;
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
      limit: row.limit,
      rate: row.rateBp === null ? '' : String(row.rateBp / 100).replace('.', ','),
      monthRate: ''
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

  /** Сохраняет черновик: поля категории, затем лимит или процент «с начала месяца экрана». */
  async save(): Promise<boolean> {
    const d = this.draft;
    const name = d.name.trim();
    if (name === '') {
      this.fieldError = 'Название не должно быть пустым';
      return false;
    }
    const isSavings = d.kind === 'savings';
    const rateBp = isSavings && d.rate.trim() !== '' ? parsePercentBp(d.rate) : null;
    if (isSavings && d.rate.trim() !== '' && rateBp === null) {
      this.fieldError = 'Процент от 0 до 100, например 14 или 14,5';
      return false;
    }
    const monthBp = isSavings && d.monthRate.trim() !== '' ? parsePercentBp(d.monthRate) : null;
    if (isSavings && d.monthRate.trim() !== '' && monthBp === null) {
      this.fieldError = 'Процент на месяц от 0 до 100';
      return false;
    }
    this.fieldError = null;
    this.saving = true;
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
      if (!isSavings && d.limit !== null && d.limit > 0 && d.limit !== row?.limit) {
        await categoriesApi.setLimit(id, this.#month, d.limit);
      }
      if (rateBp !== null && rateBp !== row?.rateBp) {
        await categoriesApi.setSavingsRate(id, this.#month, rateBp);
      }
      if (monthBp !== null) await categoriesApi.setSavingsOverride(this.#month, id, monthBp);
      await this.load(this.#month);
      this.selected = id;
      this.select(id);
      toasts.push({ message: `Сохранено: ${name}` });
      return true;
    } catch (e) {
      this.fieldError = errorText(e);
      return false;
    } finally {
      this.saving = false;
    }
  }

  /** Убирает переопределение процента на выбранный месяц: снова действует общий процент. */
  async clearMonthRate(id: number): Promise<void> {
    try {
      await categoriesApi.clearSavingsOverride(this.#month, id);
      await this.load(this.#month);
      toasts.push({ message: 'Процент месяца сброшен' });
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  async setArchived(id: number, archived: boolean): Promise<void> {
    try {
      await (archived ? categoriesApi.archive(id) : categoriesApi.unarchive(id));
      await this.load(this.#month);
      if (archived) this.selected = null;
      toasts.push({ message: archived ? 'Категория в архиве' : 'Категория возвращена' });
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
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

  /** Сдвиг на одну позицию клавиатурой (Alt+↑ / Alt+↓). */
  async nudge(id: number, delta: -1 | 1): Promise<void> {
    const visible = this.rows.map((r) => r.category.id);
    const target = visible[visible.indexOf(id) + delta];
    if (target !== undefined) await this.move(id, target);
  }

  async #reorder(ids: number[]): Promise<void> {
    const prev = this.items;
    const byId = new SvelteMap(prev.map((c) => [c.id, c]));
    this.items = ids.flatMap((id) => {
      const c = byId.get(id);
      return c ? [c] : [];
    });
    try {
      this.items = await categoriesApi.reorder(ids);
    } catch (e) {
      this.items = prev;
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  connect(): () => void {
    const offData = events.dataChanged.listen(() => {
      if (this.#month !== '') void this.load(this.#month);
    });
    return () => {
      void offData.then((off) => {
        off();
      });
    };
  }
}

function pluralCategories(n: number): string {
  const mod10 = n % 10;
  const mod100 = n % 100;
  if (mod10 === 1 && mod100 !== 11) return 'категория';
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return 'категории';
  return 'категорий';
}

const KEY = Symbol('CategoriesVm');
export const setCategoriesVm = (vm: CategoriesVm) => setContext(KEY, vm);
export const getCategoriesVm = () => getContext<CategoriesVm>(KEY);

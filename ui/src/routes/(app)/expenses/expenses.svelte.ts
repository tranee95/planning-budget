import { getContext, setContext } from 'svelte';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import {
  type CategoryDto,
  type LimitRowDto,
  type MonthOverviewDto,
  type TransactionDto,
  type TxStatusDto
} from '$lib/api/bindings';
import { searchApi, summaryApi, transactionsApi } from '$lib/api/data';
import { STATUS_ORDER } from '$lib/components/status';
import { QueryFilter } from '$lib/filters/query-filter.svelte';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';
import { connectScreen } from '$lib/stores/connect-screen';
import { tags } from '$lib/stores/tags.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';

export type SortColumn = 'date' | 'month' | 'title' | 'category' | 'amount' | 'status';

export interface CategoryBlock {
  category: CategoryDto;
  rows: TransactionDto[];
  limit: LimitRowDto | null;
}

/** ViewModel экрана «Расходы»: траты месяца, сводка лимитов, режим, фильтр и выбор строки. */
export class ExpensesVm {
  overview = $state.raw<MonthOverviewDto | null>(null);
  transactions = $state.raw<TransactionDto[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  mode = $state<'blocks' | 'table'>('blocks');
  /**
   * Запрос панели фильтров (только «Таблица»). Пока он применён, строки приходят из `tx_search`
   * по всем месяцам, а не из `tx_list` месяца; месяц ограничивает только токен `период:`.
   */
  filter = new QueryFilter();
  /** Трата, у которой открыт выбор тегов. */
  tagEditId = $state<number | null>(null);
  /** Пусто — показаны все статусы. */
  statusFilter = new SvelteSet<TxStatusDto>();
  selected = $state<number | null>(null);
  sort = $state<{ column: SortColumn; descending: boolean } | null>(null);
  /** Строки, отмеченные в таблице для массовых действий. */
  checked = new SvelteSet<number>();
  editorOpen = $state(false);
  /** Строка, открытая для инлайн-правки. */
  editingId = $state<number | null>(null);
  /** Категория, в карточке которой открыта строка добавления. */
  addingTo = $state<number | null>(null);

  #month = '';
  #req = 0;
  #pendingEdit: { id: number; after: number } | null = null;

  /** Месяц `YYYY-MM`, который показан на экране. */
  get month(): string {
    return this.#month;
  }

  visible = $derived(
    this.statusFilter.size === 0
      ? this.transactions
      : this.transactions.filter((t) => this.statusFilter.has(t.status))
  );

  /** Карточки категорий: строки уже отфильтрованы, лимиты и факт пришли из Rust. */
  blocks = $derived.by<CategoryBlock[]>(() => {
    const limits = new SvelteMap(this.overview?.limits.map((l) => [l.categoryId, l]));
    return categories.items.map((category) => ({
      category,
      rows: this.visible.filter((t) => t.categoryId === category.id),
      limit: limits.get(category.id) ?? null
    }));
  });

  /** Итог над сеткой: все три числа считает Rust. */
  totals = $derived(
    this.overview === null
      ? null
      : {
          spent: this.overview.summary.expenses,
          limits: this.overview.limitsTotal,
          remaining: this.overview.limitsRemaining
        }
  );

  /** Строки таблицы: фильтр статусов и сортировка; без сортировки — порядок ввода. */
  sortedRows = $derived.by<TransactionDto[]>(() => {
    const sort = this.sort;
    if (sort === null) return this.visible;
    const names = new SvelteMap(categories.items.map((c) => [c.id, c.name]));
    const text = (a: string, b: string) => a.localeCompare(b, 'ru');
    const compare: Record<SortColumn, (a: TransactionDto, b: TransactionDto) => number> = {
      date: (a, b) => (a.date ?? '￿').localeCompare(b.date ?? '￿'),
      month: (a, b) => a.month.localeCompare(b.month),
      title: (a, b) => text(a.title, b.title),
      category: (a, b) => text(names.get(a.categoryId) ?? '', names.get(b.categoryId) ?? ''),
      amount: (a, b) => a.amount - b.amount,
      status: (a, b) => STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status)
    };
    const sign = sort.descending ? -1 : 1;
    return [...this.visible].sort((a, b) => sign * compare[sort.column](a, b));
  });

  /** Выбранные строки, которые не скрыты фильтром: массовые действия работают только с ними. */
  checkedVisible = $derived(this.visible.filter((t) => this.checked.has(t.id)).map((t) => t.id));

  /** Запрос применяется к списку: таблица показывает результат поиска, а не месяц. */
  filtered = $derived(this.mode === 'table' && this.filter.active);

  isEmpty = $derived(!this.loading && this.error === null && this.transactions.length === 0);
  hasSelection = $derived(this.selected !== null);

  async load(month: string): Promise<void> {
    const req = ++this.#req;
    if (month !== this.#month) this.checked.clear();
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const [overview, transactions] = await Promise.all([
        summaryApi.month(month),
        this.#rows(month, req),
        categories.ensure(),
        tags.ensure()
      ]);
      if (req !== this.#req) return;
      this.overview = overview;
      this.transactions = transactions;
      this.#resolvePendingEdit(req);
    } catch (e) {
      if (req === this.#req) this.error = errorText(e);
    } finally {
      if (req === this.#req) this.loading = false;
    }
  }

  /** Строки экрана: траты месяца или, при применённом запросе, результат поиска по всем месяцам. */
  async #rows(month: string, req: number): Promise<TransactionDto[]> {
    const query = this.mode === 'table' ? this.filter.applied : '';
    if (query === '') {
      if (req === this.#req) this.filter.meta = null;
      return transactionsApi.list(month);
    }
    const found = await searchApi.transactions(query, req);
    if (req === this.#req) {
      this.filter.meta = {
        query,
        spans: found.spans,
        hints: found.hints,
        total: found.total,
        sum: found.sum,
        truncated: found.truncated
      };
    }
    return found.items;
  }

  /** Заменяет теги траты и перечитывает строки: колонка «Теги» показывает ответ Rust. */
  async setTags(id: number, tagIds: number[]): Promise<void> {
    try {
      await transactionsApi.setTags(id, tagIds);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    this.tagEditId = null;
    await this.load(this.#month);
  }

  toggleStatus(status: TxStatusDto): void {
    if (this.statusFilter.has(status)) this.statusFilter.delete(status);
    else this.statusFilter.add(status);
  }

  select(id: number | null): void {
    this.selected = id;
  }

  openEditor(): void {
    this.editorOpen = true;
  }

  closeEditor(): void {
    this.editorOpen = false;
  }

  /** Оптимистично меняет строку; при ошибке откатывает. Итоги лимитов перечитывает у Rust. */
  async setStatus(id: number, status: TxStatusDto): Promise<void> {
    const prev = this.transactions;
    const before = prev.find((t) => t.id === id)?.status;
    this.transactions = prev.map((t) => (t.id === id ? { ...t, status } : t));
    try {
      await transactionsApi.setStatus(id, status);
    } catch (e) {
      this.transactions = prev;
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    if (before !== undefined && before !== status) {
      undoStack.record({
        label: 'смена статуса',
        undo: () => this.#applyStatus(id, before),
        redo: () => this.#applyStatus(id, status)
      });
    }
    await this.#refreshOverview();
  }

  /** Итоги лимитов читаются у Rust после записи. Сбой чтения не отменяет уже сохранённую запись. */
  async #refreshOverview(): Promise<void> {
    // Список — результат поиска: итог «Найдено» и сумма меняются вместе со строками.
    if (this.filtered) {
      await this.load(this.#month);
      return;
    }
    try {
      this.overview = await summaryApi.month(this.#month);
    } catch (e) {
      toasts.push({ kind: 'error', message: `Итоги не обновились: ${errorText(e)}` });
    }
  }

  async #applyStatus(id: number, status: TxStatusDto): Promise<void> {
    await transactionsApi.setStatus(id, status);
    await this.load(this.#month);
  }

  /** Клик по заголовку: по возрастанию → по убыванию → порядок ввода. */
  toggleSort(column: SortColumn): void {
    if (this.sort?.column !== column) this.sort = { column, descending: false };
    else if (!this.sort.descending) this.sort = { column, descending: true };
    else this.sort = null;
  }

  toggleChecked(id: number): void {
    if (this.checked.has(id)) this.checked.delete(id);
    else this.checked.add(id);
  }

  /** Выбрать все видимые строки; если все уже выбраны — снять выбор. */
  toggleAllChecked(): void {
    const all = this.visible.every((t) => this.checked.has(t.id));
    this.checked.clear();
    if (!all) for (const t of this.visible) this.checked.add(t.id);
  }

  clearChecked(): void {
    this.checked.clear();
  }

  bulkSetStatus(status: TxStatusDto): Promise<void> {
    return this.#bulk((id) => transactionsApi.setStatus(id, status));
  }

  bulkSetCategory(categoryId: number): Promise<void> {
    return this.#bulk((id) => transactionsApi.update(id, { categoryId }));
  }

  /** Удаляет выбранные строки (мягко): возврат — тостом или ⌘Z. */
  async bulkDelete(): Promise<void> {
    const ids = this.checkedVisible;
    if (ids.length === 0) return;
    const results = await Promise.allSettled(ids.map((id) => transactionsApi.remove(id)));
    const removed = ids.filter((_, i) => results[i]?.status === 'fulfilled');
    const failed = results.find((r) => r.status === 'rejected');
    if (failed) toasts.push({ kind: 'error', message: errorText(failed.reason) });
    this.checked.clear();
    await this.load(this.#month);
    if (removed.length === 0) return;
    const entry = {
      label: removed.length === 1 ? 'трата удалена' : `удалено трат: ${String(removed.length)}`,
      undo: () => this.#restore(removed),
      redo: () => this.#remove(removed)
    };
    undoStack.record(entry);
    toasts.push({
      message: removed.length === 1 ? 'Трата удалена' : `Удалено трат: ${String(removed.length)}`,
      action: {
        label: 'Отменить',
        run: () => {
          void undoStack.undo(entry);
        }
      }
    });
  }

  async #restore(ids: number[]): Promise<void> {
    await Promise.all(ids.map((id) => transactionsApi.restore(id)));
    await this.load(this.#month);
  }

  async #remove(ids: number[]): Promise<void> {
    await Promise.all(ids.map((id) => transactionsApi.remove(id)));
    await this.load(this.#month);
  }

  /** Запись создания в стек отмены: undo удаляет строку, redo возвращает ту же. */
  trackCreated(row: TransactionDto): void {
    undoStack.record({
      label: 'трата добавлена',
      undo: () => this.#remove([row.id]),
      redo: () => this.#restore([row.id])
    });
  }

  /** Применяет действие к выбранным строкам; сбои собираются в один тост, остальные строки не теряются. */
  async #bulk(action: (id: number) => Promise<unknown>): Promise<void> {
    const ids = this.checkedVisible;
    if (ids.length === 0) return;
    const results = await Promise.allSettled(ids.map(action));
    const failed = results.find((r) => r.status === 'rejected');
    if (failed) toasts.push({ kind: 'error', message: errorText(failed.reason) });
    this.checked.clear();
    await this.load(this.#month);
  }

  /** Следующий статус по кругу: оплачено → долг → незапланировано → план. */
  async cycleStatus(id: number): Promise<void> {
    const row = this.transactions.find((t) => t.id === id);
    if (!row) return;
    const next = STATUS_ORDER[(STATUS_ORDER.indexOf(row.status) + 1) % STATUS_ORDER.length];
    if (next) await this.setStatus(id, next);
  }

  /** Открывает правку траты по ссылке из палитры: сразу, если строка уже загружена, иначе после загрузки. */
  openForEdit(id: number): void {
    this.#pendingEdit = { id, after: this.#req };
    this.#resolvePendingEdit();
  }

  /**
   * Открывает правку, когда строка появилась в списке. Загрузка, начатая уже после запроса, а
   * строки не принесла (её скрыл фильтр) — запрос снимается: правка не должна «выстрелить» позже.
   */
  #resolvePendingEdit(loadedReq?: number): void {
    const pending = this.#pendingEdit;
    if (pending === null) return;
    if (this.transactions.some((t) => t.id === pending.id)) {
      this.#pendingEdit = null;
      this.startEdit(pending.id);
    } else if (loadedReq !== undefined && loadedReq > pending.after) {
      this.#pendingEdit = null;
    }
  }

  startEdit(id: number): void {
    this.addingTo = null;
    this.editingId = id;
  }

  cancelEdit(): void {
    this.editingId = null;
  }

  /** Сохраняет правку открытой строки. Пустое наименование и сумма ≤ 0 не отправляются. */
  async saveEdit(patch: { title?: string; amount?: number }): Promise<void> {
    const id = this.editingId;
    if (id === null) return;
    const title = patch.title?.trim();
    if (title === '' || (patch.amount !== undefined && patch.amount <= 0)) return;
    try {
      const { current } = await transactionsApi.update(id, {
        ...(title === undefined ? {} : { title }),
        ...(patch.amount === undefined ? {} : { amount: patch.amount })
      });
      this.transactions = this.transactions.map((t) => (t.id === id ? current : t));
      this.editingId = null;
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    await this.#refreshOverview();
  }

  startAdd(categoryId: number): void {
    this.editingId = null;
    this.addingTo = categoryId;
  }

  cancelAdd(): void {
    this.addingTo = null;
  }

  /** Создаёт трату со статусом «План» в открытой карточке. */
  async commitAdd(draft: { title: string; amount: number }): Promise<void> {
    const categoryId = this.addingTo;
    const title = draft.title.trim();
    if (categoryId === null || title === '' || draft.amount <= 0) return;
    try {
      const created = await transactionsApi.create({
        month: this.#month,
        date: null,
        categoryId,
        title,
        amount: draft.amount,
        status: 'planned',
        comment: null
      });
      this.transactions = [...this.transactions, created];
      this.addingTo = null;
      this.trackCreated(created);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    await this.#refreshOverview();
  }

  /** Подписки экрана; страница вызывает в `onMount`, возвращает cleanup. */
  connect(): () => void {
    return connectScreen({
      shortcut: {
        id: 'expenses.add',
        code: 'KeyN',
        label: 'Новая трата',
        run: () => {
          this.openEditor();
        }
      },
      month: () => this.#month,
      anyMonth: () => this.filtered,
      reload: () => void this.load(this.#month)
    });
  }
}

const KEY = Symbol('ExpensesVm');
export const setExpensesVm = (vm: ExpensesVm) => setContext(KEY, vm);
export const getExpensesVm = () => getContext<ExpensesVm>(KEY);

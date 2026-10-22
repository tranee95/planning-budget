import { getContext, setContext } from 'svelte';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import {
  type CategoryDto,
  type LimitRowDto,
  type MonthOverviewDto,
  type MonthPlanDto,
  type TransactionDto,
  type TxStatusDto
} from '$lib/api/bindings';
import { planApi, searchApi, summaryApi, transactionsApi } from '$lib/api/data';
import { STATUS_LABEL, STATUS_ORDER } from '$lib/components/status';
import { QueryFilter } from '$lib/filters/query-filter.svelte';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';
import { connectScreen } from '$lib/stores/connect-screen';
import { tags } from '$lib/stores/tags.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack, type UndoEntry } from '$lib/stores/undo.svelte';
import { shiftMonth } from '$lib/format';
import { createDebtFromTransaction } from '../debts/open';
import { sortRows, type SortColumn } from './table-sort';

export type { SortColumn };

/** «1 статья», «2 статьи», «5 статей». */
function articles(n: number): string {
  const mod10 = n % 10;
  const mod100 = n % 100;
  if (mod10 === 1 && mod100 !== 11) return `${String(n)} статья`;
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return `${String(n)} статьи`;
  return `${String(n)} статей`;
}

/** Наборы тегов равны независимо от порядка. */
function sameIds(a: number[], b: number[]): boolean {
  return a.length === b.length && a.every((id) => b.includes(id));
}

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
  /** Поле, в котором стоит курсор при открытии правки: клик по названию — название, остальное — сумма. */
  editFocus = $state<'title' | 'amount'>('amount');
  /** План месяца и наличие данных в прошлом месяце: нужны только пустому состоянию. */
  plan = $state.raw<MonthPlanDto | null>(null);
  previousHasData = $state(false);
  copyBusy = $state(false);
  /** Категория, в карточке которой открыта строка добавления. */
  addingTo = $state<number | null>(null);
  #addRequestId = crypto.randomUUID();
  #committing = false;
  #saving = false;

  #month = '';
  #req = 0;
  #pendingEdit: { id: number; after: number } | null = null;

  /** Месяц `YYYY-MM`, который показан на экране. */
  get month(): string {
    return this.#month;
  }

  /** Строки без накоплений: они показываются на «Сбережениях». */
  spendingRows = $derived(
    this.transactions.filter((t) => categories.byId.get(t.categoryId)?.kind !== 'savings')
  );

  visible = $derived(
    this.statusFilter.size === 0
      ? this.spendingRows
      : this.spendingRows.filter((t) => this.statusFilter.has(t.status))
  );

  /** Карточки категорий: строки уже отфильтрованы, лимиты и факт пришли из Rust. */
  blocks = $derived.by<CategoryBlock[]>(() => {
    const limits = new SvelteMap(this.overview?.limits.map((l) => [l.categoryId, l]));
    const all = categories.spending.map((category) => ({
      category,
      rows: this.visible.filter((t) => t.categoryId === category.id),
      limit: limits.get(category.id) ?? null
    }));
    // Под фильтром статуса пустые карточки не показываются: иначе экран выглядит сломанным
    return this.statusFilter.size === 0 ? all : all.filter((b) => b.rows.length > 0);
  });

  /** Фильтр статусов выбран, но трат с такими статусами в месяце нет. */
  statusFilterEmpty = $derived(this.statusFilter.size > 0 && this.visible.length === 0);

  /** «Нет трат со статусом «План»» / «…со статусами «План», «Долг»». */
  statusFilterEmptyText = $derived.by(() => {
    const names = STATUS_ORDER.filter((st) => this.statusFilter.has(st)).map(
      (st) => `«${STATUS_LABEL[st]}»`
    );
    return `Нет трат со ${names.length > 1 ? 'статусами' : 'статусом'} ${names.join(', ')}`;
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
  sortedRows = $derived.by<readonly TransactionDto[]>(() =>
    sortRows(this.visible, this.sort, new SvelteMap(categories.items.map((c) => [c.id, c.name])))
  );

  /** Выбранные строки, которые не скрыты фильтром: массовые действия работают только с ними. */
  checkedVisible = $derived(this.visible.filter((t) => this.checked.has(t.id)).map((t) => t.id));

  /** Запрос применяется к списку: таблица показывает результат поиска, а не месяц. */
  filtered = $derived(this.mode === 'table' && this.filter.active);

  isEmpty = $derived(!this.loading && this.error === null && this.spendingRows.length === 0);
  hasSelection = $derived(this.selected !== null);

  /** В месяце нет трат и это не результат поиска: вместо сетки и итогов показывается пустое состояние. */
  showEmpty = $derived(this.isEmpty && !this.filtered);

  /** Статей расходов в плане месяца (суммы плана больше нуля). */
  planItems = $derived(
    this.plan === null
      ? 0
      : this.plan.rows.filter(
          (r) => r.plan > 0 && categories.spending.some((c) => c.id === r.categoryId)
        ).length
  );

  emptyHint = $derived(this.planItems > 0 ? `В плане ${articles(this.planItems)}` : null);

  noCategories = $derived(categories.loaded && categories.spending.length === 0);

  /** План можно перенести: он ещё не заполнен и не зафиксирован, а в прошлом месяце есть данные. */
  canCopyPlan = $derived(
    !this.noCategories &&
      this.previousHasData &&
      this.plan !== null &&
      !this.plan.locked &&
      this.plan.planExpenses === 0
  );

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
      await this.#loadEmptyContext(month, req);
    } catch (e) {
      if (req === this.#req) this.error = errorText(e);
    } finally {
      if (req === this.#req) this.loading = false;
    }
  }

  /**
   * Для пустого месяца читает план и итог прошлого месяца. Это подсказки: сбой чтения не мешает
   * экрану, кнопка «Скопировать план» просто не появится.
   */
  async #loadEmptyContext(month: string, req: number): Promise<void> {
    if (this.spendingRows.length > 0 || this.filtered) return;
    try {
      const [plan, previous] = await Promise.all([
        planApi.month(month),
        summaryApi.month(shiftMonth(month, -1))
      ]);
      if (req !== this.#req) return;
      this.plan = plan;
      this.previousHasData = previous.summary.expenses > 0;
    } catch {
      if (req !== this.#req) return;
      this.plan = null;
      this.previousHasData = false;
    }
  }

  /** Переносит план прошлого месяца в пустой; ошибка — тостом. */
  async copyPlan(): Promise<void> {
    if (this.copyBusy) return;
    this.copyBusy = true;
    try {
      await planApi.copyFromPrevious(this.#month);
    } catch (e) {
      toasts.fail(e, () => {
        void this.copyPlan();
      });
      return;
    } finally {
      this.copyBusy = false;
    }
    await this.load(this.#month);
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
    const before = this.transactions.find((t) => t.id === id)?.tagIds;
    try {
      await transactionsApi.setTags(id, tagIds);
    } catch (e) {
      toasts.fail(e, () => {
        void this.setTags(id, tagIds);
      });
      return;
    }
    this.tagEditId = null;
    if (before !== undefined && !sameIds(before, tagIds)) {
      undoStack.record({
        label: 'смена тегов',
        undo: () => this.#applyTags(id, before),
        redo: () => this.#applyTags(id, tagIds)
      });
    }
    await this.load(this.#month);
  }

  async #applyTags(id: number, tagIds: number[]): Promise<void> {
    await transactionsApi.setTags(id, tagIds);
    await this.load(this.#month);
  }

  /** «Все»: фильтр по статусам снят. */
  clearStatus(): void {
    this.statusFilter.clear();
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
      toasts.fail(e, () => {
        void this.setStatus(id, status);
      });
      return;
    }
    if (status === 'debt' && before !== undefined && before !== 'debt') {
      toasts.push({
        message: 'Трата оплачена в долг. Создать долг с графиком погашения?',
        action: {
          label: 'Создать долг',
          run: () => {
            createDebtFromTransaction(id, this.#month);
          }
        }
      });
    }
    if (before !== undefined && before !== status) {
      this.#recordWithToast(
        {
          label: 'смена статуса',
          undo: () => this.#applyStatus(id, before),
          redo: () => this.#applyStatus(id, status)
        },
        `Статус: ${STATUS_LABEL[status]}`,
        'tx-status'
      );
    }
    await this.#refreshOverview();
  }

  /** Запись в стек отмены и тост с «Отменить»; тост с тем же ключом заменяет прежний. */
  #recordWithToast(entry: UndoEntry, message: string, key: string): void {
    undoStack.record(entry);
    toasts.push({
      message,
      key,
      action: {
        label: 'Отменить',
        run: () => {
          void undoStack.undo(entry);
        }
      }
    });
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
    this.checked.clear();
    await this.#deleteIds(ids);
  }

  /** Удаляет одну сумму (крестик в строке); выбор других строк не меняется. */
  async deleteOne(id: number): Promise<void> {
    this.checked.delete(id);
    await this.#deleteIds([id]);
  }

  async #deleteIds(ids: number[]): Promise<void> {
    const results = await Promise.allSettled(ids.map((id) => transactionsApi.remove(id)));
    const removed = ids.filter((_, i) => results[i]?.status === 'fulfilled');
    const failed = results.find((r) => r.status === 'rejected');
    if (failed) toasts.fail(failed.reason);
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
    if (failed) toasts.fail(failed.reason);
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

  /** Открывает правку строки; без `focus` курсор встаёт в сумму (Enter на строке, ссылка из палитры). */
  startEdit(id: number, focus: 'title' | 'amount' = 'amount'): void {
    this.addingTo = null;
    this.editFocus = focus;
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
    if (title === '' || (patch.amount !== undefined && patch.amount <= 0) || this.#saving) return;
    const row = this.transactions.find((t) => t.id === id);
    if (row && (title ?? row.title) === row.title && (patch.amount ?? row.amount) === row.amount) {
      this.editingId = null;
      return;
    }
    this.#saving = true;
    try {
      const { current, previous } = await transactionsApi.update(id, {
        ...(title === undefined ? {} : { title }),
        ...(patch.amount === undefined ? {} : { amount: patch.amount })
      });
      this.transactions = this.transactions.map((t) => (t.id === id ? current : t));
      this.editingId = null;
      this.#recordEdit(previous, current);
    } catch (e) {
      toasts.fail(e);
      return;
    } finally {
      this.#saving = false;
    }
    await this.#refreshOverview();
  }

  /** Правка названия или суммы: в стек отмены и тост «Сумма изменена» / «Строка изменена». */
  #recordEdit(previous: TransactionDto, current: TransactionDto): void {
    const apply = (to: TransactionDto) => async (): Promise<void> => {
      await transactionsApi.update(to.id, { title: to.title, amount: to.amount });
      await this.load(this.#month);
    };
    const titleChanged = previous.title !== current.title;
    this.#recordWithToast(
      { label: 'правка строки', undo: apply(previous), redo: apply(current) },
      titleChanged ? 'Строка изменена' : 'Сумма изменена',
      'tx-edit'
    );
  }

  startAdd(categoryId: number): void {
    this.editingId = null;
    this.addingTo = categoryId;
  }

  cancelAdd(): void {
    this.addingTo = null;
  }

  /** Создаёт трату со статусом «План» в открытой карточке; без наименования берётся название категории. */
  async commitAdd(draft: { title: string; amount: number }): Promise<void> {
    const categoryId = this.addingTo;
    if (categoryId === null || draft.amount <= 0 || this.#committing) return;
    const title = draft.title.trim() || categories.byId.get(categoryId)?.name || '';
    if (title === '') return;
    // Enter и уход фокуса приходят подряд: вторая отправка до ответа первой не нужна.
    this.#committing = true;
    try {
      const created = await transactionsApi.create(this.#addRequestId, {
        month: this.#month,
        date: null,
        categoryId,
        title,
        amount: draft.amount,
        status: this.overview?.planLocked === true ? 'unplanned' : 'planned',
        comment: null
      });
      this.#addRequestId = crypto.randomUUID();
      this.transactions = [...this.transactions, created];
      this.addingTo = null;
      this.trackCreated(created);
    } catch (e) {
      toasts.fail(e);
      return;
    } finally {
      this.#committing = false;
    }
    await this.#refreshOverview();
  }

  /** Подписки экрана; страница вызывает в `onMount`, возвращает cleanup. */
  connect(): () => void {
    return connectScreen({
      month: () => this.#month,
      anyMonth: () => this.filtered,
      reload: () => void this.load(this.#month)
    });
  }
}

const KEY = Symbol('ExpensesVm');
export const setExpensesVm = (vm: ExpensesVm) => setContext(KEY, vm);
export const getExpensesVm = () => getContext<ExpensesVm>(KEY);

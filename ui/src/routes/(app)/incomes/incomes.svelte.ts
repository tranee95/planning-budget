import { getContext, setContext } from 'svelte';
import { type IncomeDto, type IncomeStatusDto, type MonthSummaryDto } from '$lib/api/bindings';
import { incomesApi, searchApi, summaryApi } from '$lib/api/data';
import { QueryFilter } from '$lib/filters/query-filter.svelte';
import { monthRange } from '$lib/format';
import { errorText } from '$lib/i18n/errors';
import { connectScreen } from '$lib/stores/connect-screen';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';

/** ViewModel экрана «Доходы»: список месяца, фильтр по статусу, создание, смена статуса. */
export class IncomesVm {
  incomes = $state.raw<IncomeDto[]>([]);
  /** Сводка месяца от Rust: итоги доходов считает `core`, а не экран. */
  summary = $state.raw<MonthSummaryDto | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);

  /** `null` — все статусы. */
  statusFilter = $state<IncomeStatusDto | null>(null);
  editorOpen = $state(false);
  /** Доход, который правят в редакторе; `null` — редактор создаёт новый. */
  editing = $state.raw<IncomeDto | null>(null);
  /** Запрос панели фильтров: пока он применён, список — результат поиска по всем месяцам. */
  filter = new QueryFilter();
  filtered = $derived(this.filter.active);

  #month = '';
  #req = 0;

  /** Месяц `YYYY-MM`, который показан на экране. */
  get month(): string {
    return this.#month;
  }

  visible = $derived(
    this.statusFilter === null
      ? this.incomes
      : this.incomes.filter((i) => i.status === this.statusFilter)
  );
  /** Итоги месяца по всем доходам, без учёта фильтра. */
  totals = $derived({
    received: this.summary?.incomeReceived ?? 0,
    expected: this.summary?.incomeExpected ?? 0,
    total: this.summary?.income ?? 0
  });
  isEmpty = $derived(!this.loading && this.error === null && this.incomes.length === 0);

  async load(month: string): Promise<void> {
    const req = ++this.#req;
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const [incomes, overview] = await Promise.all([
        this.#rows(month, req),
        summaryApi.month(month)
      ]);
      if (req !== this.#req) return;
      this.incomes = incomes;
      this.summary = overview.summary;
    } catch (e) {
      if (req === this.#req) this.error = errorText(e);
    } finally {
      if (req === this.#req) this.loading = false;
    }
  }

  /** Доходы месяца или, при применённом запросе, результат поиска по всем месяцам. */
  async #rows(month: string, req: number): Promise<IncomeDto[]> {
    const query = this.filter.applied;
    if (query === '') {
      if (req === this.#req) this.filter.meta = null;
      return incomesApi.list(month);
    }
    const found = await searchApi.incomes(query, req);
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

  openEditor(): void {
    this.editing = null;
    this.editorOpen = true;
  }

  /** Открывает редактор с текущими значениями дохода (клик по строке). */
  openEdit(id: number): void {
    const income = this.incomes.find((i) => i.id === id);
    if (income === undefined) return;
    this.editing = income;
    this.editorOpen = true;
  }

  closeEditor(): void {
    this.editorOpen = false;
    this.editing = null;
  }

  /** Сохраняет редактор: правка открытого дохода или создание нового. */
  submit(draft: {
    sourceName: string;
    amount: number;
    date: string | null;
    status: IncomeStatusDto;
  }): Promise<boolean> {
    return this.editing === null ? this.create(draft) : this.#update(this.editing, draft);
  }

  async #update(
    before: IncomeDto,
    draft: { sourceName: string; amount: number; date: string | null; status: IncomeStatusDto }
  ): Promise<boolean> {
    const sourceName = draft.sourceName.trim();
    if (sourceName === '' || draft.amount <= 0) return false;
    if (draft.date !== null) {
      const { first, last } = monthRange(before.month);
      if (draft.date < first || draft.date > last) {
        toasts.push({ kind: 'error', message: 'Дата должна быть в этом месяце' });
        return false;
      }
    }
    const patch = { sourceName, amount: draft.amount, date: draft.date, status: draft.status };
    try {
      await incomesApi.update(before.id, patch);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    }
    this.closeEditor();
    const previous = {
      sourceName: before.sourceName,
      amount: before.amount,
      date: before.date,
      status: before.status
    };
    undoStack.record({
      label: 'доход изменён',
      undo: () => this.#applyPatch(before.id, previous),
      redo: () => this.#applyPatch(before.id, patch)
    });
    toasts.push({ message: `Сохранено: ${sourceName}` });
    await this.load(this.#month);
    return true;
  }

  async #applyPatch(
    id: number,
    patch: { sourceName: string; amount: number; date: string | null; status: IncomeStatusDto }
  ): Promise<void> {
    await incomesApi.update(id, patch);
    await this.load(this.#month);
  }

  /** Удаляет доход (мягко): вернуть можно тостом или ⌘Z. */
  async remove(id: number): Promise<void> {
    const income = this.incomes.find((i) => i.id === id);
    try {
      await incomesApi.remove(id);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    const entry = {
      label: `доход «${income?.sourceName ?? ''}» удалён`,
      undo: () => this.#toggleDeleted(id, false),
      redo: () => this.#toggleDeleted(id, true)
    };
    undoStack.record(entry);
    toasts.push({
      message: 'Доход удалён',
      action: {
        label: 'Вернуть',
        run: () => {
          void undoStack.undo(entry);
        }
      }
    });
    await this.load(this.#month);
  }

  /** Создаёт доход в месяце экрана. Пустой источник и сумма ≤ 0 не отправляются. */
  async create(draft: {
    sourceName: string;
    amount: number;
    date: string | null;
    status: IncomeStatusDto;
  }): Promise<boolean> {
    const sourceName = draft.sourceName.trim();
    if (sourceName === '' || draft.amount <= 0) return false;
    if (draft.date !== null) {
      const { first, last } = monthRange(this.#month);
      if (draft.date < first || draft.date > last) {
        toasts.push({ kind: 'error', message: 'Дата должна быть в этом месяце' });
        return false;
      }
    }
    try {
      const created = await incomesApi.create({
        month: this.#month,
        date: draft.date,
        sourceName,
        amount: draft.amount,
        status: draft.status,
        comment: null
      });
      this.incomes = [...this.incomes, created];
      this.editorOpen = false;
      undoStack.record({
        label: 'доход добавлен',
        undo: () => this.#toggleDeleted(created.id, true),
        redo: () => this.#toggleDeleted(created.id, false)
      });
      toasts.push({ message: `Добавлено: ${sourceName}` });
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    }
    await this.#refreshSummary();
    return true;
  }

  async #toggleDeleted(id: number, deleted: boolean): Promise<void> {
    await (deleted ? incomesApi.remove(id) : incomesApi.restore(id));
    await this.load(this.#month);
  }

  /** Оптимистично меняет статус; при ошибке откатывает и показывает тост. */
  async setStatus(id: number, status: IncomeStatusDto): Promise<void> {
    const prev = this.incomes;
    const before = prev.find((i) => i.id === id)?.status;
    this.incomes = prev.map((i) => (i.id === id ? { ...i, status } : i));
    try {
      await incomesApi.update(id, { status });
    } catch (e) {
      this.incomes = prev;
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    if (before !== undefined && before !== status) {
      undoStack.record({
        label: 'смена статуса дохода',
        undo: () => this.#applyStatus(id, before),
        redo: () => this.#applyStatus(id, status)
      });
    }
    await this.#refreshSummary();
  }

  /** Итоги читаются у Rust после записи. Сбой чтения не отменяет сохранённую запись. */
  async #refreshSummary(): Promise<void> {
    // Список — результат поиска: итог «Найдено» и сумма меняются вместе со строками.
    if (this.filtered) {
      await this.load(this.#month);
      return;
    }
    try {
      this.summary = (await summaryApi.month(this.#month)).summary;
    } catch (e) {
      toasts.push({ kind: 'error', message: `Итоги не обновились: ${errorText(e)}` });
    }
  }

  async #applyStatus(id: number, status: IncomeStatusDto): Promise<void> {
    await incomesApi.update(id, { status });
    await this.load(this.#month);
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

const KEY = Symbol('IncomesVm');
export const setIncomesVm = (vm: IncomesVm) => setContext(KEY, vm);
export const getIncomesVm = () => getContext<IncomesVm>(KEY);

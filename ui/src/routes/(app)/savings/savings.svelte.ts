import { getContext, setContext } from 'svelte';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { SavingsProgressDto, TransactionDto } from '$lib/api/bindings';
import { LoadStamp, onDataChanged } from '$lib/api/data-events';
import { categoriesApi, planApi, savingsApi, transactionsApi } from '$lib/api/data';
import { CATEGORY_COLORS } from '$lib/category-colors';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';

/** Название суммы, если пользователь его не ввёл. */
export const DEFAULT_AMOUNT_TITLE = 'Накопление';

/** Накопление и его суммы за выбранный месяц. */
export type SavingCard = {
  categoryId: number;
  name: string;
  /** Суммы месяца в порядке ввода: каждая — обычная оплаченная трата в категории накопления. */
  amounts: TransactionDto[];
  /** Сумма строк месяца, копейки. */
  total: number;
  /** План на месяц, копейки (из `plan_month`); 0 — не задан. */
  plan: number;
};

/** Закрытая цель: накопление в архиве; его суммы остаются в итогах и на графике. */
export type ClosedGoal = { categoryId: number; name: string; total: number };

/**
 * ViewModel раздела «Сбережения». У каждого накопления на выбранный месяц свой список сумм:
 * новая сумма — это строка в карточке (как трата в карточке категории на «Расходах»), её можно
 * изменить и удалить. Откладывали 30 000 ₽ тремя разами по 10 000 ₽ — три строки одной
 * карточки. Итоги месяца и «Накоплено всего» считает Rust.
 */
export class SavingsVm {
  month = $state('');
  /** Сумма плана месяца из `plan_month` (то, что показано как «Откладываю в месяц»). */
  planTotal = $state(0);
  /** Накоплено за весь период и взносы по месяцам (считает Rust). */
  progress = $state.raw<SavingsProgressDto | null>(null);
  cards = $state.raw<SavingCard[]>([]);
  /** Закрытые цели (накопления в архиве) и их накоплено всего. */
  closed = $state.raw<ClosedGoal[]>([]);
  showClosed = $state(false);
  /** Цель, которую собрались закрыть: ждёт подтверждения. */
  closeAsk = $state<{ id: number; name: string } | null>(null);
  loading = $state(true);
  error = $state('');
  newName = $state('');
  adding = $state(false);

  /** Карточка, в которую сейчас вводится новая сумма. */
  addingTo = $state<number | null>(null);
  draftTitle = $state('');
  draftAmount = $state<number | null>(null);
  /** Строка, которую правят. */
  editingId = $state<number | null>(null);
  saving = $state(false);
  /** Накопление, чьё название правят (клик по названию в карточке). */
  renamingId = $state<number | null>(null);

  #stamp = new LoadStamp();
  /** Номер последней загрузки: ответы устаревших запросов отбрасываются. */
  #generation = 0;
  #addRequestId = crypto.randomUUID();
  /** Цвета всех категорий (в том числе закрытых): новому накоплению берётся ещё не занятый. */
  #usedColors: string[] = [];

  /** Отложено в выбранном месяце по всем накоплениям, в том числе по закрытым целям. */
  monthTotal = $state(0);

  /** Цвета рядов графика: у закрытых целей категории нет в общем списке, цвет берётся из ответа. */
  chartColors = $derived(
    new SvelteMap((this.progress?.categories ?? []).map((c) => [c.categoryId, c.color]))
  );

  async load(month: string): Promise<void> {
    this.month = month;
    this.#stamp.mark();
    const generation = ++this.#generation;
    this.error = '';
    try {
      const [rows, plan, progress, everyCategory] = await Promise.all([
        transactionsApi.list(month),
        planApi.month(month),
        savingsApi.progress(month),
        categoriesApi.list(true),
        categories.ensure()
      ]);
      if (generation !== this.#generation) return;
      const planByCategory = new SvelteMap(plan.rows.map((r) => [r.categoryId, r.plan]));
      this.planTotal = plan.planSavings;
      this.progress = progress;
      this.#usedColors = everyCategory.map((c) => c.color);
      const totals = new SvelteMap(progress.categories.map((c) => [c.categoryId, c.total]));
      const savingsIds = new SvelteSet(
        everyCategory.filter((c) => c.kind === 'savings').map((c) => c.id)
      );
      this.monthTotal = rows
        .filter((t) => savingsIds.has(t.categoryId) && t.status !== 'planned')
        .reduce((sum, t) => sum + t.amount, 0);
      this.closed = everyCategory
        .filter((c) => c.kind === 'savings' && c.archived)
        .map((c) => ({ categoryId: c.id, name: c.name, total: totals.get(c.id) ?? 0 }));
      this.cards = categories.items
        .filter((c) => c.kind === 'savings' && !c.archived)
        .map((category) => {
          // Плановые строки старых данных в карточку не попадают: это не отложенные суммы.
          const amounts = rows.filter(
            (t) => t.categoryId === category.id && t.status !== 'planned'
          );
          return {
            categoryId: category.id,
            name: category.name,
            amounts,
            total: amounts.reduce((sum, t) => sum + t.amount, 0),
            plan: planByCategory.get(category.id) ?? 0
          };
        });
    } catch (e) {
      if (generation === this.#generation) this.error = errorText(e);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  // ---- добавление суммы ----

  startAdd(categoryId: number): void {
    // Строка ввода уже открыта: введённое не сбрасывается.
    if (this.addingTo === categoryId) return;
    this.editingId = null;
    this.addingTo = categoryId;
    this.draftTitle = '';
    this.draftAmount = null;
  }

  cancelAdd(): void {
    this.addingTo = null;
    this.draftTitle = '';
    this.draftAmount = null;
    this.#addRequestId = crypto.randomUUID();
  }

  get canCommitAdd(): boolean {
    return this.addingTo !== null && this.draftAmount !== null && this.draftAmount > 0;
  }

  /** Сохраняет введённую сумму как новую строку накопления за выбранный месяц. */
  async commitAdd(again = false): Promise<boolean> {
    const categoryId = this.addingTo;
    const amount = this.draftAmount;
    if (categoryId === null || amount === null || amount <= 0 || this.saving) return false;
    this.saving = true;
    try {
      const created = await transactionsApi.create(this.#addRequestId, {
        month: this.month,
        date: null,
        categoryId,
        title: this.draftTitle.trim() === '' ? DEFAULT_AMOUNT_TITLE : this.draftTitle.trim(),
        amount,
        status: 'paid',
        comment: null
      });
      if (again) {
        // Строка ввода остаётся: следующая сумма — новый запрос, поэтому новый идентификатор.
        this.draftTitle = '';
        this.draftAmount = null;
        this.#addRequestId = crypto.randomUUID();
      } else {
        this.cancelAdd();
      }
      undoStack.record({
        label: 'сумма добавлена',
        undo: () => this.#removeIds([created.id]),
        redo: () => this.#restoreIds([created.id])
      });
      await this.load(this.month);
      return true;
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    } finally {
      this.saving = false;
    }
  }

  // ---- правка и удаление ----

  startEdit(id: number): void {
    this.addingTo = null;
    this.editingId = id;
  }

  cancelEdit(): void {
    this.editingId = null;
  }

  async saveEdit(patch: { title: string; amount: number }): Promise<boolean> {
    const id = this.editingId;
    if (id === null || this.saving) return false;
    const before = this.cards.flatMap((c) => c.amounts).find((t) => t.id === id);
    if (before === undefined) return false;
    const title = patch.title.trim() === '' ? DEFAULT_AMOUNT_TITLE : patch.title.trim();
    if (patch.amount <= 0) {
      toasts.push({ kind: 'error', message: 'Сумма должна быть больше нуля' });
      return false;
    }
    this.editingId = null;
    if (title === before.title && patch.amount === before.amount) return true;
    this.saving = true;
    try {
      await transactionsApi.update(id, { title, amount: patch.amount });
      undoStack.record({
        label: 'сумма изменена',
        undo: () => this.#patch(id, { title: before.title, amount: before.amount }),
        redo: () => this.#patch(id, { title, amount: patch.amount })
      });
      await this.load(this.month);
      return true;
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    } finally {
      this.saving = false;
    }
  }

  /** Удаляет сумму (мягко): вернуть можно тостом или ⌘Z. */
  async remove(id: number): Promise<void> {
    try {
      await transactionsApi.remove(id);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    if (this.editingId === id) this.editingId = null;
    const entry = {
      label: 'сумма удалена',
      undo: () => this.#restoreIds([id]),
      redo: () => this.#removeIds([id])
    };
    undoStack.record(entry);
    toasts.push({
      message: 'Сумма удалена',
      action: {
        label: 'Вернуть',
        run: () => {
          void undoStack.undo(entry);
        }
      }
    });
    await this.load(this.month);
  }

  // ---- переименование (Issue #137) ----

  /** Открывает правку названия; закрытые цели (нет в карточках) не переименовываются. */
  startRename(categoryId: number): void {
    if (!this.cards.some((c) => c.categoryId === categoryId)) return;
    this.addingTo = null;
    this.editingId = null;
    this.renamingId = categoryId;
  }

  cancelRename(): void {
    this.renamingId = null;
  }

  /**
   * Сохраняет новое название накопления (`categories_update`). Пустое и занятое имя отклоняет
   * Rust; ошибка показывается тостом, прежнее имя остаётся. ⌘Z и «Отменить» возвращают прежнее.
   */
  async rename(categoryId: number, raw: string): Promise<boolean> {
    this.renamingId = null;
    const before = this.cards.find((c) => c.categoryId === categoryId)?.name;
    if (before === undefined) return false;
    const name = raw.trim();
    if (name === before) return true;
    try {
      await categoriesApi.update(categoryId, { name });
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    }
    const entry = {
      label: 'накопление переименовано',
      undo: () => this.#setName(categoryId, before),
      redo: () => this.#setName(categoryId, name)
    };
    undoStack.record(entry);
    toasts.push({
      message: `Переименовано: «${name}»`,
      action: {
        label: 'Отменить',
        run: () => {
          void undoStack.undo(entry);
        }
      }
    });
    categories.reset();
    await this.load(this.month);
    return true;
  }

  async #setName(categoryId: number, name: string): Promise<void> {
    await categoriesApi.update(categoryId, { name });
    categories.reset();
    await this.load(this.month);
  }

  // ---- закрытие цели ----

  askClose(categoryId: number): void {
    const card = this.cards.find((c) => c.categoryId === categoryId);
    if (card === undefined) return;
    this.closeAsk = { id: categoryId, name: card.name };
  }

  cancelClose(): void {
    this.closeAsk = null;
  }

  /** Закрывает цель: накопление уходит в архив, его суммы остаются в итогах и на графике. */
  async confirmClose(): Promise<void> {
    const ask = this.closeAsk;
    if (ask === null) return;
    this.closeAsk = null;
    try {
      await categoriesApi.archive(ask.id);
      categories.reset();
      toasts.push({ message: 'Цель закрыта' });
      await this.load(this.month);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  /** Возвращает закрытую цель в список накоплений. */
  async reopen(categoryId: number): Promise<void> {
    try {
      await categoriesApi.unarchive(categoryId);
      categories.reset();
      await this.load(this.month);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  // ---- план и новые накопления ----

  /** План накопления на выбранный месяц и дальше (пока не изменят); 0 убирает план. */
  async setPlan(categoryId: number, amount: number): Promise<boolean> {
    try {
      await savingsApi.setFixedPlan(categoryId, this.month, amount);
      await this.load(this.month);
      return true;
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    }
  }

  /** Новое накопление: категория типа «Сбережения», скрытая на экранах «Категории» и «Расходы». */
  async add(): Promise<boolean> {
    const name = this.newName.trim();
    if (name === '' || this.adding) return false;
    this.adding = true;
    try {
      await categoriesApi.create({
        name,
        kind: 'savings',
        color: this.#nextColor(),
        note: null
      });
      this.newName = '';
      categories.reset();
      await this.load(this.month);
      return true;
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return false;
    } finally {
      this.adding = false;
    }
  }

  /** Первый цвет палитры, не занятый другой категорией; когда палитра кончилась — по кругу. */
  #nextColor(): string {
    const free = CATEGORY_COLORS.find((c) => !this.#usedColors.includes(c));
    return free ?? CATEGORY_COLORS[this.#usedColors.length % CATEGORY_COLORS.length] ?? '#3AA567';
  }

  /** Пересчёт при изменении записей и настроек. Возвращает cleanup. */
  connect(): () => void {
    return onDataChanged((change) => {
      if (this.#stamp.isFresh(change) || this.month === '') return;
      void this.load(this.month);
    });
  }

  async #removeIds(ids: number[]): Promise<void> {
    await Promise.all(ids.map((id) => transactionsApi.remove(id)));
    await this.load(this.month);
  }

  async #restoreIds(ids: number[]): Promise<void> {
    await Promise.all(ids.map((id) => transactionsApi.restore(id)));
    await this.load(this.month);
  }

  async #patch(id: number, patch: { title: string; amount: number }): Promise<void> {
    await transactionsApi.update(id, patch);
    await this.load(this.month);
  }
}

const KEY = Symbol('savings-vm');
export const setSavingsVm = (vm: SavingsVm) => setContext(KEY, vm);
export const getSavingsVm = () => getContext<SavingsVm>(KEY);

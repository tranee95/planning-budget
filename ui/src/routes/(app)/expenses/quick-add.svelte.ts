import { SvelteMap } from 'svelte/reactivity';
import type { CategoryDto, TitleSuggestionDto, TxStatusDto } from '$lib/api/bindings';
import { transactionsApi } from '$lib/api/data';
import { formatMoney, monthRange } from '$lib/format';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';
import type { ExpensesVm } from './expenses.svelte';

export type Suggestion = TitleSuggestionDto;

/** Пауза после ввода перед запросом подсказок. */
const SUGGEST_DEBOUNCE_MS = 70;

function localToday(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${String(now.getFullYear())}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/**
 * ViewModel быстрого добавления траты (N). Подсказки наименований и частота категорий берутся
 * из всей истории (Rust: `tx_suggest`, `tx_category_usage`), а не из трат открытого месяца.
 */
export class QuickAddVm {
  amount = $state<number | null>(null);
  /** Категория: выбирает пользователь; пока он не выбрал, подставляется самая частая. */
  get categoryId(): number | null {
    return this.#categoryId;
  }
  set categoryId(value: number | null) {
    this.#categoryId = value;
    this.#chosen = true;
  }
  #categoryId = $state<number | null>(null);
  #chosen = false;
  /** Была попытка сохранить: ошибки заполнения показываются только после неё. */
  #attempted = $state(false);
  /** Форме нужно вернуть фокус в поле суммы («ещё одну»). */
  focusAmount = $state(false);
  /** Наименование: ввод запускает запрос подсказок после паузы. */
  get title(): string {
    return this.#title;
  }
  set title(value: string) {
    this.#title = value;
    this.#requestSuggestions(value);
  }
  #title = $state('');
  /** Подсказки из истории; пусто, пока нет ввода или ответа. */
  suggestions = $state.raw<Suggestion[]>([]);
  /** Число трат по категориям за последние месяцы. */
  usage = $state.raw<SvelteMap<number, number>>(new SvelteMap());
  #timer: ReturnType<typeof setTimeout> | undefined;
  #suggestReq = 0;
  /** `YYYY-MM-DD` или пустая строка. */
  date = $state('');
  status = $state<TxStatusDto>('paid');
  saving = $state(false);
  /** Один id на попытку сохранить: повторная отправка той же траты не создаст дубль. */
  #requestId = crypto.randomUUID();

  #expenses: ExpensesVm;

  constructor(expenses: ExpensesVm, today: () => string = localToday) {
    this.#expenses = expenses;
    this.#preselect();
    const day = today();
    this.date = day.startsWith(expenses.month) ? day : '';
    // В зафиксированном месяце новая трата по умолчанию вне плана.
    if (expenses.overview?.planLocked === true) this.status = 'unplanned';
    void transactionsApi
      .categoryUsage()
      .then((rows) => {
        this.usage = new SvelteMap(rows.map((r) => [r.categoryId, r.uses]));
        this.#preselect();
      })
      .catch(() => {
        // Без частоты категории остаются в порядке настроек.
      });
  }

  /** Подставляет самую частую категорию, пока пользователь сам не выбрал. */
  #preselect(): void {
    if (this.#chosen) return;
    this.#categoryId = this.chips[0]?.id ?? null;
  }

  /** Снимает выбор категории (для проверки ошибки «Выберите категорию»). */
  clearCategory(): void {
    this.#categoryId = null;
    this.#chosen = true;
  }

  /** Категории: чаще используемые — раньше, при равенстве — в порядке настроек. */
  chips = $derived.by<CategoryDto[]>(() =>
    [...categories.spending].sort(
      (a, b) => (this.usage.get(b.id) ?? 0) - (this.usage.get(a.id) ?? 0)
    )
  );

  /** Дата вне месяца экрана: строка попала бы в один месяц, а датирована другим. */
  /** Дата набрана не до конца или не существует: пока так, трату не сохранить. */
  dateValid = $state(true);

  dateError = $derived.by<string | null>(() => {
    if (!this.dateValid) return 'Укажите дату полностью: дд.мм.гггг';
    if (this.date === '') return null;
    const { first, last } = monthRange(this.#expenses.month);
    return this.date < first || this.date > last ? 'Дата должна быть в этом месяце' : null;
  });

  canSave = $derived(
    this.amount !== null && this.amount > 0 && this.categoryId !== null && this.dateError === null
  );

  /** Что мешает сохранить; показывается после первой попытки и пропадает, когда поле исправлено. */
  formError = $derived.by<string | null>(() => {
    if (!this.#attempted) return null;
    if (this.amount === null || this.amount <= 0) return 'Введите сумму';
    if (this.categoryId === null) return 'Выберите категорию';
    return null;
  });

  /**
   * Предпросмотр лимита по мере ввода: остаток категории из сводки Rust минус вводимая сумма.
   * Итоговые значения после сохранения снова приходят из Rust.
   */
  limitWarning = $derived.by<{ category: string; over: number } | null>(() => {
    if (this.categoryId === null || this.amount === null) return null;
    const row = this.#expenses.overview?.limits.find((l) => l.categoryId === this.categoryId);
    if (row?.remaining == null) return null;
    const over = this.amount - row.remaining;
    if (over <= 0) return null;
    return { category: categories.byId.get(this.categoryId)?.name ?? '', over };
  });

  #requestSuggestions(query: string): void {
    clearTimeout(this.#timer);
    const req = ++this.#suggestReq;
    if (query.trim() === '') {
      this.suggestions = [];
      return;
    }
    this.#timer = setTimeout(() => {
      transactionsApi
        .suggest(query)
        .then((found) => {
          if (req === this.#suggestReq) this.suggestions = found;
        })
        .catch(() => {
          if (req === this.#suggestReq) this.suggestions = [];
        });
    }, SUGGEST_DEBOUNCE_MS);
  }

  dispose(): void {
    clearTimeout(this.#timer);
    this.#suggestReq += 1;
  }

  pickSuggestion(suggestion: Suggestion): void {
    clearTimeout(this.#timer);
    this.#suggestReq += 1;
    this.suggestions = [];
    this.#title = suggestion.title;
    this.categoryId = suggestion.categoryId;
  }

  /** Сохраняет трату. `another` — остаться в форме для следующей (⇧Enter). */
  async save(another: boolean): Promise<void> {
    if (this.saving) return;
    if (!this.canSave) {
      this.#attempted = true;
      return;
    }
    const { amount, categoryId } = this;
    if (amount === null || categoryId === null) return;
    // Без наименования трата называется по категории, как в карточке на «Расходах».
    const title = this.title.trim() || (categories.byId.get(categoryId)?.name ?? '');
    this.saving = true;
    try {
      const created = await transactionsApi.create(this.#requestId, {
        month: this.#expenses.month,
        date: this.date === '' ? null : this.date,
        categoryId,
        title,
        amount,
        status: this.status,
        comment: null
      });
      this.#requestId = crypto.randomUUID();
      this.#expenses.trackCreated(created);
      toasts.push({ message: `Добавлено: ${title}, ${formatMoney(amount)}`, kind: 'success' });
      await this.#expenses.load(this.#expenses.month);
      if (another) {
        this.amount = null;
        this.title = '';
        this.#attempted = false;
        this.focusAmount = true;
      } else {
        this.#expenses.closeEditor();
      }
    } catch (e) {
      toasts.fail(e, () => {
        void this.save(another);
      });
    } finally {
      this.saving = false;
    }
  }
}

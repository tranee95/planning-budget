import { getContext, setContext } from 'svelte';
import type {
  DebtDto,
  DebtPatchDto,
  DebtScheduleKindDto,
  DebtsOverviewDto,
  SchedulePaymentDto,
  TransactionDto
} from '$lib/api/bindings';
import { LoadStamp, onDataChanged } from '$lib/api/data-events';
import { debtsApi, transactionsApi } from '$lib/api/data';
import { errorText } from '$lib/i18n/errors';
import { toasts } from '$lib/stores/toasts.svelte';
import { categories } from '$lib/stores/categories.svelte';

export type Draft = {
  lender: string;
  /** Копейки; `null` — поле пусто. */
  amount: number | null;
  takenMonth: string;
  categoryId: number | null;
  comment: string;
  kind: 'equal' | 'single';
  /** Количество месяцев текстом, как вводит пользователь. */
  months: string;
  singleMonth: string;
};

const emptyDraft = (month: string): Draft => ({
  lender: '',
  amount: null,
  takenMonth: month,
  categoryId: null,
  comment: '',
  kind: 'equal',
  months: '3',
  singleMonth: month
});

/** ViewModel экрана «Долги»: список, форма долга, быстрый график (его считает Rust). */
export class DebtsVm {
  overview = $state.raw<DebtsOverviewDto | null>(null);
  includeClosed = $state(false);
  loading = $state(false);
  error = $state<string | null>(null);

  /** `null` — форма закрыта; `'new'` — создание. */
  selected = $state<number | 'new' | null>(null);
  draft = $state<Draft>(emptyDraft(''));
  /** График для сохранения: ответ `debt_schedule_preview`. */
  schedule = $state.raw<SchedulePaymentDto[]>([]);
  /** Сумма графика, копейки: приходит от `debt_schedule_preview`, UI её не считает. */
  scheduleTotal = $state(0);
  scheduleError = $state<string | null>(null);
  /** Трата, из которой создаётся долг (сумма, статья и месяц берутся из неё). */
  fromTransaction = $state.raw<TransactionDto | null>(null);
  saving = $state(false);
  fieldError = $state<string | null>(null);

  #month = '';
  #req = 0;
  #stamp = new LoadStamp();
  #scheduleReq = 0;
  /** Ключ графика при открытии долга: пока он не изменился, график берётся из `payments`. */
  #openedKey: string | null = null;

  get month(): string {
    return this.#month;
  }

  current = $derived(
    typeof this.selected === 'number'
      ? (this.overview?.debts.find((d) => d.id === this.selected) ?? null)
      : null
  );

  /** График можно пересобрать, пока у долга нет оплаченных строк. */
  scheduleEditable = $derived(
    this.selected === 'new' ||
      (this.current?.payments.every((p) => p.status === 'planned') ?? false)
  );

  /** Параметры быстрого графика из формы. */
  scheduleKind = $derived.by<DebtScheduleKindDto | null>(() => {
    if (this.draft.kind === 'single') return { kind: 'single', month: this.draft.singleMonth };
    const months = Number(this.draft.months);
    return Number.isInteger(months) ? { kind: 'equalParts', months } : null;
  });

  /** Ключ входа графика: страница пересчитывает график, когда он меняется. */
  scheduleKey = $derived(
    JSON.stringify([this.draft.amount, this.draft.takenMonth, this.scheduleKind])
  );

  canSave = $derived(
    this.draft.lender.trim() !== '' &&
      this.draft.amount !== null &&
      this.draft.amount > 0 &&
      (!this.scheduleEditable || (this.schedule.length > 0 && this.scheduleError === null)) &&
      !this.saving
  );

  subtitle = $derived.by(() => {
    const o = this.overview;
    if (o === null) return '';
    if (o.openCount === 0) return 'Открытых долгов нет';
    return `${String(o.openCount)} открытых`;
  });

  async load(month: string): Promise<void> {
    this.#stamp.mark();
    const req = ++this.#req;
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const [overview] = await Promise.all([
        debtsApi.list(month, this.includeClosed),
        categories.ensure()
      ]);
      if (req !== this.#req) return;
      this.overview = overview;
      if (
        typeof this.selected === 'number' &&
        !overview.debts.some((d) => d.id === this.selected)
      ) {
        this.selected = null;
      }
    } catch (e) {
      if (req === this.#req) this.error = errorText(e);
    } finally {
      if (req === this.#req) this.loading = false;
    }
  }

  setIncludeClosed(value: boolean): Promise<void> {
    this.includeClosed = value;
    return this.load(this.#month);
  }

  startNew(): void {
    this.selected = 'new';
    this.fromTransaction = null;
    this.fieldError = null;
    this.scheduleError = null;
    this.schedule = [];
    this.#openedKey = null;
    this.draft = emptyDraft(this.#month);
  }

  /** Долг из траты со статусом «Долг»: сумма, статья и месяц подставляются из неё. */
  async startFromTransaction(txId: number, month: string): Promise<void> {
    try {
      const rows = await transactionsApi.list(month);
      const tx = rows.find((t) => t.id === txId);
      if (!tx) return;
      this.startNew();
      this.fromTransaction = tx;
      this.draft = {
        ...emptyDraft(tx.month),
        amount: tx.amount,
        categoryId: tx.categoryId,
        lender: ''
      };
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  select(id: number): void {
    const debt = this.overview?.debts.find((d) => d.id === id);
    if (!debt) return;
    this.selected = id;
    this.fromTransaction = null;
    this.fieldError = null;
    this.scheduleError = null;
    this.schedule = debt.payments.map((p) => ({ month: p.month, amount: p.amount }));
    // График сохранённого долга всегда сходится с суммой долга (проверяет core).
    this.scheduleTotal = debt.amount;
    this.draft = {
      lender: debt.lender,
      amount: debt.amount,
      takenMonth: debt.takenMonth,
      categoryId: debt.categoryId,
      comment: debt.comment ?? '',
      kind: 'equal',
      months: String(Math.max(debt.payments.length, 1)),
      singleMonth: debt.payments.at(-1)?.month ?? debt.takenMonth
    };
    this.#openedKey = this.scheduleKey;
  }

  close(): void {
    this.selected = null;
    this.fromTransaction = null;
    this.fieldError = null;
  }

  /** Пересчитывает быстрый график у Rust; устаревшие ответы отбрасываются. */
  async refreshSchedule(): Promise<void> {
    if (!this.scheduleEditable) return;
    const req = ++this.#scheduleReq;
    if (this.#openedKey !== null && this.scheduleKey === this.#openedKey) {
      // Поля графика не менялись (или вернулись к исходным): график долга как есть.
      this.schedule = (this.current?.payments ?? []).map((p) => ({
        month: p.month,
        amount: p.amount
      }));
      this.scheduleError = null;
      return;
    }
    const amount = this.draft.amount;
    const kind = this.scheduleKind;
    if (amount === null || amount <= 0 || kind === null) {
      this.schedule = [];
      this.scheduleError = null;
      return;
    }
    try {
      const preview = await debtsApi.schedulePreview(amount, this.draft.takenMonth, kind);
      if (req !== this.#scheduleReq) return;
      this.schedule = preview.rows;
      this.scheduleTotal = preview.total;
      this.scheduleError = null;
    } catch (e) {
      if (req !== this.#scheduleReq) return;
      this.schedule = [];
      this.scheduleError = errorText(e);
    }
  }

  async save(): Promise<void> {
    const amount = this.draft.amount;
    if (!this.canSave || amount === null) return;
    this.saving = true;
    this.fieldError = null;
    try {
      const d = this.draft;
      const comment = d.comment.trim() === '' ? null : d.comment.trim();
      if (this.selected === 'new') {
        if (this.fromTransaction !== null) {
          await debtsApi.fromTransaction(this.fromTransaction.id, d.lender, [...this.schedule]);
        } else {
          await debtsApi.create({
            lender: d.lender,
            amount,
            takenMonth: d.takenMonth,
            takenDate: null,
            categoryId: d.categoryId,
            comment,
            schedule: [...this.schedule]
          });
        }
      } else if (typeof this.selected === 'number') {
        const patch: DebtPatchDto = {
          lender: d.lender,
          categoryId: d.categoryId,
          comment
        };
        if (this.scheduleEditable) {
          patch.amount = amount;
          patch.schedule = [...this.schedule];
        }
        await debtsApi.update(this.selected, patch);
      }
      this.close();
      await this.load(this.#month);
    } catch (e) {
      this.fieldError = errorText(e);
    } finally {
      this.saving = false;
    }
  }

  /** Мягкое удаление с возвратом тостом «Вернуть». */
  async remove(debt: DebtDto): Promise<void> {
    try {
      await debtsApi.remove(debt.id);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
      return;
    }
    this.close();
    toasts.push({
      message: `Долг «${debt.lender}» удалён`,
      action: {
        label: 'Вернуть',
        run: () => {
          void debtsApi
            .restore(debt.id)
            .then(() => this.load(this.#month))
            .catch((e: unknown) => {
              toasts.push({ kind: 'error', message: errorText(e) });
            });
        }
      }
    });
    await this.load(this.#month);
  }

  /** Отметка строки графика: оплачено (с сегодняшней датой) или снова в плане. */
  async setPayment(paymentId: number, paid: boolean, today: string): Promise<void> {
    try {
      await debtsApi.setPaymentStatus(paymentId, paid ? 'paid' : 'planned', paid ? today : null);
      await this.load(this.#month);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  /** Подписки экрана; страница вызывает в `onMount`, возвращает cleanup. */
  connect(): () => void {
    return onDataChanged((change) => {
      if (this.#stamp.isFresh(change)) return;
      if (this.#month !== '') void this.load(this.#month);
    });
  }
}

const KEY = Symbol('DebtsVm');
export const setDebtsVm = (vm: DebtsVm) => setContext(KEY, vm);
export const getDebtsVm = () => getContext<DebtsVm>(KEY);

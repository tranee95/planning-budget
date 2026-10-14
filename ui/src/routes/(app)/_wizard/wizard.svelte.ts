import type {
  CategoryDto,
  MonthPlanDto,
  PlanWizardInputDto,
  WizardSavingsDto
} from '$lib/api/bindings';
import { planApi, savingsApi } from '$lib/api/data';
import { currentMonth, formatMonth } from '$lib/format';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';
import { onboarding } from '$lib/stores/onboarding.svelte';
import { toasts } from '$lib/stores/toasts.svelte';

export const WIZARD_STEPS = ['Доход', 'Статьи', 'Сбережения', 'Итог'] as const;

export type IncomeRow = { name: string; amount: number | null };
export type LineRow = { category: CategoryDto; amount: number | null };
export type SavingsRow = {
  category: CategoryDto;
  /** Сумма в месяц, копейки; `null` — не задана. */
  fixed: number | null;
};

/** ViewModel мастера первого месяца: ввод собирается здесь, «Не распределено» считает Rust. */
export class WizardVm {
  readonly month: string;
  readonly monthName: string;
  step = $state(0);
  incomes = $state<IncomeRow[]>([{ name: 'Зарплата', amount: null }]);
  lines = $state<LineRow[]>([]);
  savings = $state<SavingsRow[]>([]);
  preview = $state.raw<MonthPlanDto | null>(null);
  loading = $state(true);
  applying = $state(false);
  error = $state('');
  /** Месяц уже спланирован (в том числе автопереносом): мастер заполняет только пустой месяц. */
  blockedReason = $state<string | null>(null);
  /** План превышает доход, и пользователь видит вопрос «Всё равно зафиксировать?». */
  confirmingOver = $state(false);

  #previewReq = 0;

  /** Месяц плана: по умолчанию текущий календарный. */
  constructor(month: string = currentMonth()) {
    this.month = month;
    this.monthName = formatMonth(month);
  }

  /** Ввод в виде, который понимает бэкенд: пустые и нулевые строки отбрасываются. */
  input = $derived.by<PlanWizardInputDto>(() => ({
    incomes: this.incomes
      .filter((i) => i.name.trim() !== '' && (i.amount ?? 0) > 0)
      .map((i) => ({ sourceName: i.name.trim(), amount: i.amount ?? 0 })),
    lines: this.lines
      .filter((l) => (l.amount ?? 0) > 0)
      .map((l) => ({ categoryId: l.category.id, amount: l.amount ?? 0 })),
    // Пустое поле — это 0: явная сумма перекрывает прежний процент плана, который в UI не виден.
    savings: this.savings.map((s): WizardSavingsDto => ({
      categoryId: s.category.id,
      plan: { kind: 'fixed', amount: s.fixed ?? 0 }
    }))
  }));

  /** Ключ ввода: страница пересчитывает «Не распределено», когда он меняется. */
  inputKey = $derived(JSON.stringify(this.input));

  get last(): boolean {
    return this.step === WIZARD_STEPS.length - 1;
  }

  /** Строки итога: статьи, у которых после ввода есть план. */
  planLines = $derived.by(() => {
    return (this.preview?.rows ?? [])
      .filter((r) => r.plan !== 0)
      .map((r) => ({ name: categories.byId.get(r.categoryId)?.name ?? '', plan: r.plan }));
  });

  async init(): Promise<void> {
    this.loading = true;
    try {
      const [overview, current] = await Promise.all([
        savingsApi.overview(Number(this.month.slice(0, 4))),
        planApi.month(this.month),
        // Не ensure(): после знакомства стор мог загрузиться до засева и остаться пустым.
        categories.load()
      ]);
      this.blockedReason =
        current.locked || current.hasPlannedLines
          ? `На ${this.monthName} уже есть план. Мастер заполняет только пустой месяц: суммы меняются на «Расходах», зафиксированный план — на «Обзоре» кнопкой «Разблокировать».`
          : null;
      const active = categories.items.filter((c) => !c.archived);
      this.lines = active
        .filter((c) => c.kind !== 'savings')
        .map((category) => ({ category, amount: null }));
      this.savings = active
        .filter((c) => c.kind === 'savings')
        .map((category) => {
          const item = overview.items.find((i) => i.categoryId === category.id);
          // Процент плана (старый сейф) не переносится: до ввода дохода он даёт 0.
          return { category, fixed: item?.planFixed ?? null };
        });
      await this.refreshPreview();
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.loading = false;
    }
  }

  /** Пересчёт «Не распределено» у Rust; устаревшие ответы отбрасываются. */
  async refreshPreview(): Promise<void> {
    const req = ++this.#previewReq;
    try {
      const preview = await planApi.preview(this.month, this.input);
      if (req === this.#previewReq) this.preview = preview;
    } catch (e) {
      if (req === this.#previewReq) this.error = errorText(e);
    }
  }

  addIncome(): void {
    this.incomes = [...this.incomes, { name: '', amount: null }];
  }

  removeIncome(index: number): void {
    this.incomes = this.incomes.filter((_, i) => i !== index);
  }

  /** Шаг «Доход» пропустить нельзя: без дохода «Не распределено» ничего не значит. */
  get canNext(): boolean {
    if (this.blockedReason !== null || this.last) return false;
    return this.step !== 0 || this.input.incomes.length > 0;
  }

  /** На сколько план превышает доход (копейки); 0, если не превышает. */
  get overBy(): number {
    const unallocated = this.preview?.unallocated ?? 0;
    return unallocated < 0 ? -unallocated : 0;
  }

  next(): void {
    if (this.canNext) this.step += 1;
  }

  back(): void {
    this.confirmingOver = false;
    if (this.step > 0) this.step -= 1;
  }

  /** «Назад к плану»: отказ от фиксации плана, который превышает доход. */
  cancelOver(): void {
    this.confirmingOver = false;
  }

  /**
   * «План готов»: доходы, плановые траты и накопления записываются одной командой.
   * Если план превышает доход, первый вызов только просит подтверждения, второй записывает.
   */
  async apply(): Promise<void> {
    if (this.applying || this.blockedReason !== null) return;
    if (this.overBy > 0 && !this.confirmingOver) {
      this.confirmingOver = true;
      return;
    }
    this.applying = true;
    this.error = '';
    try {
      await planApi.wizardApply(this.month, this.input);
      toasts.push({ message: `План на ${this.monthName} готов` });
      onboarding.planApplied();
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.applying = false;
    }
  }
}

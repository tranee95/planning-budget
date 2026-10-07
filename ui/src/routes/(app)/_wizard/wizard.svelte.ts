import type {
  CategoryDto,
  MonthPlanDto,
  PlanWizardInputDto,
  WizardSavingsDto
} from '$lib/api/bindings';
import { planApi, savingsApi } from '$lib/api/data';
import { parsePercentBp } from '$lib/category-colors';
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
  kind: 'percent' | 'fixed';
  /** Процент текстом, как вводит пользователь. */
  percent: string;
  fixed: number | null;
};

const percentText = (bp: number): string => String(bp / 100).replace('.', ',');

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
    savings: this.savings.flatMap((s): WizardSavingsDto[] => {
      if (s.kind === 'fixed') {
        return s.fixed === null
          ? []
          : [{ categoryId: s.category.id, plan: { kind: 'fixed', amount: s.fixed } }];
      }
      const rateBp = parsePercentBp(s.percent);
      return rateBp === null
        ? []
        : [{ categoryId: s.category.id, plan: { kind: 'percent', rateBp } }];
    })
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
      const [overview] = await Promise.all([
        savingsApi.overview(Number(this.month.slice(0, 4))),
        categories.ensure()
      ]);
      const active = categories.items.filter((c) => !c.archived);
      this.lines = active
        .filter((c) => c.kind !== 'savings')
        .map((category) => ({ category, amount: null }));
      this.savings = active
        .filter((c) => c.kind === 'savings')
        .map((category) => {
          const item = overview.items.find((i) => i.categoryId === category.id);
          return {
            category,
            kind: item?.planKind ?? 'percent',
            percent: percentText(item?.planRateBp ?? 0),
            fixed: item?.planFixed ?? null
          };
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

  next(): void {
    if (!this.last) this.step += 1;
  }

  back(): void {
    if (this.step > 0) this.step -= 1;
  }

  /** «План готов»: доходы, плановые траты и накопления записываются одной командой. */
  async apply(): Promise<void> {
    if (this.applying) return;
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

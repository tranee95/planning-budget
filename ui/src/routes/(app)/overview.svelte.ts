import { getContext, setContext } from 'svelte';
import {
  type CategoryDto,
  type LimitRowDto,
  type MonthOverviewDto,
  type MonthPlanDto,
  type PlanRowDto,
  type MonthSummaryDto,
  type SeriesRangeDto,
  type SettingsDto,
  type YearSummaryDto
} from '$lib/api/bindings';
import { onDataChanged } from '$lib/api/data-events';
import { debtsApi, planApi, settingsApi, summaryApi } from '$lib/api/data';
import { formatMoney, formatMonth, formatPercent, today } from '$lib/format';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';

export type ChartRange = SeriesRangeDto;

export type LimitLine = { category: CategoryDto; row: LimitRowDto };

export type PlanLine = { category: CategoryDto; row: PlanRowDto };

/** ViewModel экрана «Обзор»: сводка месяца, лимиты и ряд для графика. Суммы считает Rust. */
export class OverviewVm {
  overview = $state.raw<MonthOverviewDto | null>(null);
  /** Годовая сводка года выбранного месяца: средние для подсказок. */
  year = $state.raw<YearSummaryDto | null>(null);
  settings = $state.raw<SettingsDto | null>(null);
  series = $state.raw<MonthSummaryDto[]>([]);
  /** План месяца: «Не распределено», план → факт (все числа из Rust). */
  plan = $state.raw<MonthPlanDto | null>(null);
  planBusy = $state(false);
  range = $state<ChartRange>('year');
  loading = $state(false);
  error = $state<string | null>(null);

  #month = '';
  #req = 0;
  #seriesReq = 0;

  get month(): string {
    return this.#month;
  }

  summary = $derived(this.overview?.summary ?? null);

  /** Лимиты месяца по убыванию использования; категории без лимита и расходов в список не попадают. */
  limitLines = $derived.by<LimitLine[]>(() => {
    const lines: LimitLine[] = [];
    for (const row of this.overview?.limits ?? []) {
      const category = categories.byId.get(row.categoryId);
      if (category && row.limit !== null && category.kind !== 'savings') {
        lines.push({ category, row });
      }
    }
    return lines.sort((a, b) => (b.row.usage ?? 0) - (a.row.usage ?? 0));
  });

  overCount = $derived(this.overview?.overCount ?? 0);

  /** Строки «план → факт»: статьи, у которых есть план или факт. */
  planLines = $derived.by<PlanLine[]>(() => {
    const lines: PlanLine[] = [];
    for (const row of this.plan?.rows ?? []) {
      const category = categories.byId.get(row.categoryId);
      if (category && (row.plan !== 0 || row.fact !== 0)) lines.push({ category, row });
    }
    return lines;
  });

  /** «Скопировать план из прошлого месяца» предлагается, пока плановых трат нет. */
  canCopyPlan = $derived(this.plan !== null && !this.plan.locked && this.plan.planExpenses === 0);

  /** Подзаголовок экрана — инсайт месяца. */
  subtitle = $derived.by(() => {
    const name = formatMonth(this.#month, 'short');
    if (this.summary === null) return name;
    if (this.limitLines.length === 0) return `${name}: лимиты не заданы`;
    if (this.overCount === 0) return `${name}: все категории в лимитах`;
    return `${name}: ${plural(this.overCount, ['категория', 'категории', 'категорий'])} выше лимита`;
  });

  savingsHint = $derived.by(() => {
    const s = this.summary;
    if (s === null || s.corridor === 'noIncome') return 'нет дохода в месяце';
    const rate = formatPercent(s.savingsRateBp ?? 0);
    if (s.corridor === 'below') {
      const min = this.settings === null ? '' : ` ${formatPercent(this.settings.savingsMinBp)}`;
      return `${rate} дохода · до${min} не хватает ${formatMoney(s.topUpToMin)}`;
    }
    return `${rate} дохода · ${s.corridor === 'within' ? 'в коридоре' : 'выше коридора'}`;
  });

  incomeHint = $derived.by(() => {
    const s = this.summary;
    if (s === null) return '';
    return s.incomeExpected > 0
      ? `ожидается ${formatMoney(s.incomeExpected)}`
      : s.income > 0
        ? 'всё получено'
        : 'доходов нет';
  });

  async load(month: string): Promise<void> {
    const req = ++this.#req;
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const yearNumber = Number(month.slice(0, 4));
      const [overview, year, settings, plan] = await Promise.all([
        summaryApi.month(month),
        summaryApi.year(yearNumber),
        settingsApi.get(),
        planApi.month(month),
        categories.ensure()
      ]);
      const series = await summaryApi.series(month, this.range);
      if (req !== this.#req) return;
      this.overview = overview;
      this.year = year;
      this.settings = settings;
      this.plan = plan;
      this.series = series;
    } catch (e) {
      if (req === this.#req) this.error = errorText(e);
    } finally {
      if (req === this.#req) this.loading = false;
    }
  }

  async setRange(range: ChartRange): Promise<void> {
    if (range === this.range) return;
    this.range = range;
    if (this.#month === '') return;
    const req = ++this.#seriesReq;
    try {
      const series = await summaryApi.series(this.#month, range);
      if (req === this.#seriesReq) this.series = series;
    } catch (e) {
      if (req === this.#seriesReq) this.error = errorText(e);
    }
  }

  /** «План готов», «Разблокировать», «Скопировать план»: действие, затем перечитать экран. */
  async #planAction(action: (month: string) => Promise<MonthPlanDto>): Promise<void> {
    if (this.#month === '' || this.planBusy) return;
    this.planBusy = true;
    try {
      this.plan = await action(this.#month);
      await this.load(this.#month);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    } finally {
      this.planBusy = false;
    }
  }

  /** Погашение долга из плана месяца: оплачено (сегодняшней датой) или снова в плане. */
  async payRepayment(paymentId: number, paid: boolean): Promise<void> {
    if (this.#month === '' || this.planBusy) return;
    this.planBusy = true;
    try {
      await debtsApi.setPaymentStatus(paymentId, paid ? 'paid' : 'planned', paid ? today() : null);
      await this.load(this.#month);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    } finally {
      this.planBusy = false;
    }
  }

  lockPlan(): Promise<void> {
    return this.#planAction(planApi.lock);
  }

  unlockPlan(): Promise<void> {
    return this.#planAction(planApi.unlock);
  }

  copyPlan(): Promise<void> {
    return this.#planAction(planApi.copyFromPrevious);
  }

  /** Подписки экрана; страница вызывает в `onMount`, возвращает cleanup. */
  connect(): () => void {
    return onDataChanged(({ months }) => {
      const year = this.#month.slice(0, 4);
      const affects = months.length === 0 || months.some((m) => m.startsWith(year));
      if (affects && this.#month !== '') void this.load(this.#month);
    });
  }
}

function plural(n: number, forms: [string, string, string]): string {
  const mod10 = n % 10;
  const mod100 = n % 100;
  const form =
    mod10 === 1 && mod100 !== 11
      ? forms[0]
      : mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)
        ? forms[1]
        : forms[2];
  return `${String(n)} ${form}`;
}

const KEY = Symbol('OverviewVm');
export const setOverviewVm = (vm: OverviewVm) => setContext(KEY, vm);
export const getOverviewVm = () => getContext<OverviewVm>(KEY);

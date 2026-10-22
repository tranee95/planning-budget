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
import { LoadStamp, onDataChanged } from '$lib/api/data-events';
import { debtsApi, planApi, settingsApi, summaryApi } from '$lib/api/data';
import { cachedCall, dayKey, resultCache } from '$lib/api/result-cache';
import {
  formatMoney,
  formatMonth,
  formatPercent,
  formatMonthLower,
  formatPercentWhole,
  formatWeeks,
  today
} from '$lib/format';
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
  #stamp = new LoadStamp();
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

  /** Траты, которые уже выше плана (не сбережения): «+4 800 ₽» против плана у категории. */
  overPlanCount = $derived(
    this.planLines.filter((l) => l.category.kind !== 'savings' && l.row.deviation > 0).length
  );

  /** В месяце нет ни плана, ни фактов, ни дохода: говорить про лимиты рано. */
  emptyMonth = $derived(
    this.summary !== null &&
      this.planLines.length === 0 &&
      this.summary.income === 0 &&
      this.summary.expenses === 0
  );

  /** Подзаголовок экрана — инсайт месяца: сначала лимиты, затем план. */
  subtitle = $derived.by(() => {
    const name = formatMonth(this.#month, 'short');
    if (this.summary === null) return name;
    if (this.emptyMonth) return `План на ${name.toLowerCase()} ещё не составлен`;
    if (this.overCount > 0)
      return `${name}: ${plural(this.overCount, ['категория', 'категории', 'категорий'])} на лимите или выше`;
    if (this.overPlanCount > 0)
      return `${name}: ${plural(this.overPlanCount, ['категория', 'категории', 'категорий'])} выше плана`;
    if (this.limitLines.length === 0) return `${name}: лимиты не заданы`;
    return `${name}: все категории в лимитах и плане`;
  });

  savingsHint = $derived.by(() => {
    const s = this.summary;
    if (s === null || s.corridor === 'noIncome') return 'нет дохода в месяце';
    const rate = formatPercentWhole(s.savingsRateBp ?? 0);
    if (s.corridor === 'below') {
      const min = this.settings === null ? '' : ` ${formatPercent(this.settings.savingsMinBp)}`;
      return `${rate} дохода · до${min} не хватает ${formatMoney(s.topUpToMin)}`;
    }
    return `${rate} дохода · ${s.corridor === 'within' ? 'в коридоре' : 'выше коридора'}`;
  });

  /** Подпись «На неделю» в текущем месяце: сколько недель осталось (считает ядро). */
  weeksHint = $derived.by(() => {
    const weeksLeft = this.summary?.weeksLeft ?? null;
    return weeksLeft === null ? null : `до конца месяца осталось ${formatWeeks(weeksLeft)} недели`;
  });

  /** Вторая строка «Свободного остатка»: без записей со статусом «План». */
  freeActualHint = $derived.by(() =>
    this.summary === null ? '' : `по факту: ${formatMoney(this.summary.freeActual)}`
  );

  incomeHint = $derived.by(() => {
    const s = this.summary;
    if (s === null) return '';
    if (s.income === 0) return 'доходов нет';
    return s.incomeExpected > 0
      ? `из ${formatMoney(s.income)} · ожидается ${formatMoney(s.incomeExpected)}`
      : `из ${formatMoney(s.income)} · всё получено`;
  });

  async load(month: string): Promise<void> {
    this.#stamp.mark();
    const req = ++this.#req;
    this.#month = month;
    this.loading = true;
    this.error = null;
    try {
      const yearNumber = Number(month.slice(0, 4));
      // Расчёты Rust берутся из кеша, пока данные не менялись; график грузится вместе с остальным.
      const day = dayKey();
      const [overview, year, settings, plan, series] = await Promise.all([
        cachedCall(`overview|${day}|month|${month}`, () => summaryApi.month(month)),
        cachedCall(`overview|${day}|year|${String(yearNumber)}`, () => summaryApi.year(yearNumber)),
        settingsApi.get(),
        cachedCall(`overview|${day}|plan|${month}`, () => planApi.month(month)),
        cachedCall(`overview|${day}|series|${month}|${this.range}`, () =>
          summaryApi.series(month, this.range)
        ),
        categories.ensure()
      ]);
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
      const month = this.#month;
      const series = await cachedCall(`overview|${dayKey()}|series|${month}|${range}`, () =>
        summaryApi.series(month, range)
      );
      if (req === this.#seriesReq) this.series = series;
    } catch (e) {
      if (req === this.#seriesReq) this.error = errorText(e);
    }
  }

  /** «План готов», «Разблокировать», «Скопировать план»: действие, затем перечитать экран. */
  async #planAction(
    action: (month: string) => Promise<MonthPlanDto>
  ): Promise<MonthPlanDto | null> {
    if (this.#month === '' || this.planBusy) return null;
    this.planBusy = true;
    try {
      const plan = await action(this.#month);
      this.plan = plan;
      // Событие об изменении может прийти позже ответа: кеш сбрасывается сразу.
      resultCache.clear();
      await this.load(this.#month);
      return plan;
    } catch (e) {
      toasts.fail(e);
      return null;
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
      resultCache.clear();
      await this.load(this.#month);
    } catch (e) {
      toasts.fail(e, () => {
        void this.payRepayment(paymentId, paid);
      });
    } finally {
      this.planBusy = false;
    }
  }

  /** «План готов»: после фиксации тост с «Разблокировать» и, если есть, строкой о нераспределённом. */
  async lockPlan(): Promise<void> {
    const month = this.#month;
    const plan = await this.#planAction(planApi.lock);
    if (plan === null) return;
    toasts.push({
      message: planLockedText(month, plan.unallocated),
      action: {
        label: 'Разблокировать',
        run: () => {
          void this.unlockPlan();
        }
      }
    });
  }

  async unlockPlan(): Promise<void> {
    await this.#planAction(planApi.unlock);
  }

  async copyPlan(): Promise<void> {
    await this.#planAction(planApi.copyFromPrevious);
  }

  /** Подписки экрана; страница вызывает в `onMount`, возвращает cleanup. */
  connect(): () => void {
    return onDataChanged((change) => {
      if (this.#stamp.isFresh(change)) return;
      const year = this.#month.slice(0, 4);
      const { months } = change;
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

/**
 * Текст тоста после «План готов». «Не распределено» берётся из ответа Rust (`plan.unallocated`):
 * положительное — доход без назначения, отрицательное — план больше дохода.
 */
export function planLockedText(month: string, unallocated: number): string {
  const head = `План на ${formatMonthLower(month)} зафиксирован`;
  if (unallocated > 0) return `${head}. Не распределено ${formatMoney(unallocated)}`;
  if (unallocated < 0) return `${head}. План превышает доход на ${formatMoney(-unallocated)}`;
  return head;
}

const KEY = Symbol('OverviewVm');
export const setOverviewVm = (vm: OverviewVm) => setContext(KEY, vm);
export const getOverviewVm = () => getContext<OverviewVm>(KEY);

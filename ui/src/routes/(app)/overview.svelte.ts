import { getContext, setContext } from 'svelte';
import {
  events,
  type CategoryDto,
  type LimitRowDto,
  type MonthOverviewDto,
  type MonthSummaryDto,
  type SettingsDto,
  type YearSummaryDto
} from '$lib/api/bindings';
import { settingsApi, summaryApi } from '$lib/api/data';
import { formatMoney, formatMonth, formatPercent } from '$lib/format';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';

export type ChartRange = 'year' | '12m' | 'all';

export type LimitLine = { category: CategoryDto; row: LimitRowDto };

/** Сколько лет назад заглядывает «Всё»: хватает на любую личную историю, запросов не больше этого. */
const MAX_YEARS_BACK = 10;

/** ViewModel экрана «Обзор»: сводка месяца, лимиты и ряд для графика. Суммы считает Rust. */
export class OverviewVm {
  overview = $state.raw<MonthOverviewDto | null>(null);
  /** Годовая сводка года выбранного месяца: средние для подсказок. */
  year = $state.raw<YearSummaryDto | null>(null);
  settings = $state.raw<SettingsDto | null>(null);
  series = $state.raw<MonthSummaryDto[]>([]);
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

  overCount = $derived(this.limitLines.filter((l) => l.row.level === 'over').length);

  /** Подзаголовок экрана — инсайт месяца. */
  subtitle = $derived.by(() => {
    const name = formatMonth(this.#month, 'short');
    if (this.summary === null) return name;
    if (this.limitLines.length === 0) return `${name}: лимиты не заданы`;
    if (this.overCount === 0) return `${name}: все категории в лимитах`;
    return `${name}: ${plural(this.overCount, ['категория', 'категории', 'категорий'])} выше лимита`;
  });

  /** Расходы месяца относительно среднего за год, в процентах; `null` — сравнивать не с чем. */
  expensesDelta = $derived.by(() => {
    const avg = this.year?.avgExpenses ?? 0;
    if (this.summary === null || avg <= 0 || (this.year?.monthsWithData ?? 0) < 2) return null;
    return Math.round(((this.summary.expenses - avg) / avg) * 100);
  });

  savingsHint = $derived.by(() => {
    const s = this.summary;
    if (s === null || s.corridor === 'noIncome') return 'нет дохода в месяце';
    const rate = formatPercent(Math.round((s.savingsRate ?? 0) * 10_000));
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
      const [overview, year, settings] = await Promise.all([
        summaryApi.month(month),
        summaryApi.year(yearNumber),
        settingsApi.get(),
        categories.ensure()
      ]);
      const series = await this.#series(month, year);
      if (req !== this.#req) return;
      this.overview = overview;
      this.year = year;
      this.settings = settings;
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
    if (this.#month === '' || this.year === null) return;
    const req = ++this.#seriesReq;
    try {
      const series = await this.#series(this.#month, this.year);
      if (req === this.#seriesReq) this.series = series;
    } catch (e) {
      if (req === this.#seriesReq) this.error = errorText(e);
    }
  }

  async #series(month: string, current: YearSummaryDto): Promise<MonthSummaryDto[]> {
    const yearNumber = current.year;
    if (this.range === 'year') return current.months;
    if (this.range === '12m') {
      const prev = await summaryApi.year(yearNumber - 1);
      return [...prev.months, ...current.months].filter((m) => m.month <= month).slice(-12);
    }
    const older = await Promise.all(
      Array.from({ length: MAX_YEARS_BACK }, (_, i) => summaryApi.year(yearNumber - i - 1))
    );
    const all = [...older.reverse(), current];
    return all.flatMap((y) => y.months).filter((m) => m.income > 0 || m.expenses > 0);
  }

  /** Подписки экрана; страница вызывает в `onMount`, возвращает cleanup. */
  connect(): () => void {
    const offData = events.dataChanged.listen(({ payload }) => {
      const year = this.#month.slice(0, 4);
      const affects = payload.months.length === 0 || payload.months.some((m) => m.startsWith(year));
      if (affects && this.#month !== '') void this.load(this.#month);
    });
    return () => {
      void offData.then((off) => {
        off();
      });
    };
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

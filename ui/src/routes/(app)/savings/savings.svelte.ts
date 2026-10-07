import { getContext, setContext } from 'svelte';
import type { AccumulationDto, SavingsOverviewDto } from '$lib/api/bindings';
import { onDataChanged } from '$lib/api/data-events';
import { categoriesApi, savingsApi } from '$lib/api/data';
import { parsePercentBp } from '$lib/category-colors';
import { currentMonth } from '$lib/format';
import { errorText } from '$lib/i18n/errors';
import { categories } from '$lib/stores/categories.svelte';
import { toasts } from '$lib/stores/toasts.svelte';

export type ParamsDraft = {
  /** Проценты текстом, как вводит пользователь: «16», «13,5». */
  rate: string;
  tax: string;
  /** Копейки; `null` — поле пусто. */
  balance: number | null;
  /** `YYYY-MM`. */
  month: string;
  planKind: 'percent' | 'fixed';
  planPercent: string;
  /** Копейки; `null` — поле пусто. */
  planFixed: number | null;
};

const percentText = (bp: number): string => String(bp / 100).replace('.', ',');

const emptyDraft = (): ParamsDraft => ({
  rate: '',
  tax: '',
  balance: null,
  month: '',
  planKind: 'percent',
  planPercent: '',
  planFixed: null
});

/** ViewModel раздела «Сбережения»: факт и прогноз считает Rust, здесь — выбор накопления, года и форма. */
export class SavingsVm {
  year = $state(Number(currentMonth().slice(0, 4)));
  data = $state.raw<SavingsOverviewDto | null>(null);
  selectedId = $state<number | null>(null);
  loading = $state(true);
  error = $state('');
  draft = $state<ParamsDraft>(emptyDraft());
  paramsError = $state('');
  saving = $state(false);

  /** Накопление идёт от стартового месяца: раньше первого года данных страницы нет. */
  minYear = $derived(this.data?.firstYear ?? this.year);
  maxYear = $derived(Number(currentMonth().slice(0, 4)));

  selected = $derived<AccumulationDto | null>(
    this.data?.items.find((i) => i.categoryId === this.selectedId) ?? null
  );

  /** Название накопления берётся из категории. */
  nameOf(categoryId: number): string {
    return categories.byId.get(categoryId)?.name ?? '';
  }

  /** Есть ли в году хотя бы один взнос: без них факт — только купоны на стартовый баланс. */
  hasContributions = $derived(this.selected?.months.some((m) => m.savings > 0) ?? false);

  /** Номер последней загрузки: ответы устаревших запросов отбрасываются. */
  #generation = 0;

  /**
   * Читает накопления. Черновик формы заполняется при смене выбора и после сохранения, чтобы
   * перезагрузка (смена года, событие данных) не стирала то, что пользователь набирает.
   */
  async load(options: { refillDraft?: boolean } = {}): Promise<void> {
    const generation = ++this.#generation;
    this.loading = true;
    this.error = '';
    try {
      const [data] = await Promise.all([savingsApi.overview(this.year), categories.ensure()]);
      if (generation !== this.#generation) return;
      this.data = data;
      const stillThere = data.items.some((i) => i.categoryId === this.selectedId);
      if (!stillThere) {
        this.selectedId = data.items[0]?.categoryId ?? null;
        this.#fillDraft();
      } else if (options.refillDraft === true) {
        this.#fillDraft();
      }
    } catch (e) {
      if (generation === this.#generation) this.error = errorText(e);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  select(categoryId: number): void {
    if (categoryId === this.selectedId) return;
    this.selectedId = categoryId;
    this.#fillDraft();
  }

  async setYear(year: number): Promise<void> {
    if (year < this.minYear || year > this.maxYear || year === this.year) return;
    this.year = year;
    await this.load();
  }

  #fillDraft(): void {
    const item = this.selected;
    if (item === null) {
      this.draft = emptyDraft();
      return;
    }
    this.draft = {
      rate: percentText(item.params.annualRateBp),
      tax: percentText(item.params.taxBp),
      balance: item.params.initialBalance,
      month: item.params.initialMonth,
      planKind: item.planKind,
      planPercent: percentText(item.planRateBp),
      planFixed: item.planFixed
    };
    this.paramsError = '';
  }

  /** Сохраняет параметры и план выбранного накопления; страница пересчитывается из Rust. */
  async saveParams(): Promise<boolean> {
    const item = this.selected;
    if (item === null) return false;
    const rate = parsePercentBp(this.draft.rate);
    const tax = parsePercentBp(this.draft.tax);
    if (rate === null || tax === null) {
      this.paramsError = 'Ставка и налог — проценты от 0 до 100, например 16 или 13,5.';
      return false;
    }
    if (this.draft.balance === null || this.draft.month === '') {
      this.paramsError = 'Укажите стартовый баланс и месяц начала накопления.';
      return false;
    }
    const planPercent =
      this.draft.planKind === 'percent' ? parsePercentBp(this.draft.planPercent) : 0;
    if (planPercent === null) {
      this.paramsError = 'План — процент от дохода от 0 до 100, например 14.';
      return false;
    }
    if (this.draft.planKind === 'fixed' && this.draft.planFixed === null) {
      this.paramsError = 'Укажите сумму плана в месяц.';
      return false;
    }
    this.saving = true;
    this.paramsError = '';
    try {
      await savingsApi.setParams(item.categoryId, {
        annualRateBp: rate,
        taxBp: tax,
        initialBalance: this.draft.balance,
        initialMonth: this.draft.month
      });
      const from = currentMonth();
      if (this.draft.planKind === 'fixed' && this.draft.planFixed !== null) {
        await savingsApi.setFixedPlan(item.categoryId, from, this.draft.planFixed);
      } else {
        await categoriesApi.setSavingsRate(item.categoryId, from, planPercent);
      }
      toasts.push({ message: 'Накопление сохранено' });
      await this.load({ refillDraft: true });
      this.year = Math.min(Math.max(this.year, this.minYear), this.maxYear);
      return true;
    } catch (e) {
      this.paramsError = errorText(e);
      return false;
    } finally {
      this.saving = false;
    }
  }

  /** Пересчёт при изменении записей и настроек. Возвращает cleanup. */
  connect(): () => void {
    return onDataChanged(() => {
      void this.load();
    });
  }
}

const KEY = Symbol('savings-vm');
export const setSavingsVm = (vm: SavingsVm) => setContext(KEY, vm);
export const getSavingsVm = () => getContext<SavingsVm>(KEY);

import { getContext, setContext } from 'svelte';
import { events, type BondsDto, type ChartDataDto, type SettingsDto } from '$lib/api/bindings';
import { bondsApi, settingsApi } from '$lib/api/data';
import { factChart, forecastChart } from '$lib/charts/bonds-charts';
import { parsePercentBp } from '$lib/category-colors';
import { errorText } from '$lib/i18n/errors';
import { currentMonth } from '$lib/format';
import { toasts } from '$lib/stores/toasts.svelte';

export type ParamsDraft = {
  /** Проценты текстом, как вводит пользователь: «16», «13,5». */
  rate: string;
  tax: string;
  /** Копейки; `null` — поле пусто. */
  balance: number | null;
  /** `YYYY-MM`. */
  month: string;
};

const percentText = (bp: number): string => String(bp / 100).replace('.', ',');

/** ViewModel страницы «Облигации»: факт и прогноз считает Rust, здесь — выбор года и параметры. */
export class BondsVm {
  year = $state(Number(currentMonth().slice(0, 4)));
  data = $state.raw<BondsDto | null>(null);
  settings = $state.raw<SettingsDto | null>(null);
  loading = $state(true);
  error = $state('');
  draft = $state<ParamsDraft>({ rate: '', tax: '', balance: null, month: '' });
  paramsError = $state('');
  saving = $state(false);

  /** Накопление идёт от стартового месяца: раньше его года страницы нет. */
  minYear = $derived(Number((this.settings?.bondsInitialMonth ?? currentMonth()).slice(0, 4)));
  maxYear = $derived(Number(currentMonth().slice(0, 4)));

  factChart = $derived<ChartDataDto | null>(
    this.data === null ? null : factChart(this.data.months)
  );
  forecastChart = $derived<ChartDataDto | null>(
    this.data === null ? null : forecastChart(this.data.year, this.data.scenarios)
  );

  /** Есть ли в году хотя бы один взнос: без них факт — только купоны на стартовый баланс. */
  hasContributions = $derived(this.data?.months.some((m) => m.savings > 0) ?? false);

  /** Номер последней загрузки: ответы устаревших запросов отбрасываются. */
  #generation = 0;

  /**
   * Читает настройки и расчёт. Настройки грузятся отдельно: если расчёт не получился (например,
   * из-за сохранённого стартового баланса), форма параметров остаётся доступной для исправления.
   * Черновик формы заполняется только при первой загрузке и после сохранения, чтобы перезагрузка
   * (смена года, событие данных) не стирала то, что пользователь набирает.
   */
  async load(options: { refillDraft?: boolean } = {}): Promise<void> {
    const generation = ++this.#generation;
    this.loading = true;
    this.error = '';
    try {
      const settings = await settingsApi.get();
      if (generation !== this.#generation) return;
      const first = this.settings === null;
      this.settings = settings;
      if (first || options.refillDraft === true) this.#fillDraft(settings);
      const data = await bondsApi.projection(this.year);
      if (generation !== this.#generation) return;
      this.data = data;
    } catch (e) {
      if (generation === this.#generation) this.error = errorText(e);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  async setYear(year: number): Promise<void> {
    if (year < this.minYear || year > this.maxYear || year === this.year) return;
    this.year = year;
    await this.load();
  }

  /** Год страницы остаётся внутри допустимых границ после смены стартового месяца. */
  #clampYear(): void {
    this.year = Math.min(Math.max(this.year, this.minYear), this.maxYear);
  }

  #fillDraft(settings: SettingsDto): void {
    this.draft = {
      rate: percentText(settings.bondsRateBp),
      tax: percentText(settings.bondsCouponTaxBp),
      balance: settings.bondsInitialBalance,
      month: settings.bondsInitialMonth
    };
    this.paramsError = '';
  }

  /** Сохраняет параметры; страница пересчитывается из Rust. */
  async saveParams(): Promise<boolean> {
    const rate = parsePercentBp(this.draft.rate);
    const tax = parsePercentBp(this.draft.tax);
    if (rate === null || tax === null) {
      this.paramsError = 'Ставка и налог — проценты от 0 до 100, например 16 или 13,5.';
      return false;
    }
    if (this.draft.balance === null || this.draft.month === '') {
      this.paramsError = 'Укажите стартовый баланс и месяц начала накоплений.';
      return false;
    }
    this.saving = true;
    this.paramsError = '';
    try {
      await settingsApi.set({
        bondsRateBp: rate,
        bondsCouponTaxBp: tax,
        bondsInitialBalance: this.draft.balance,
        bondsInitialMonth: this.draft.month
      });
      toasts.push({ message: 'Параметры облигаций сохранены' });
      this.settings = await settingsApi.get();
      this.#clampYear();
      await this.load({ refillDraft: true });
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
    const off = events.dataChanged.listen(() => {
      void this.load();
    });
    return () => {
      void off.then((stop) => {
        stop();
      });
    };
  }
}

const KEY = Symbol('bonds-vm');

export function setBondsVm(vm: BondsVm): void {
  setContext(KEY, vm);
}

export function getBondsVm(): BondsVm {
  return getContext<BondsVm>(KEY);
}

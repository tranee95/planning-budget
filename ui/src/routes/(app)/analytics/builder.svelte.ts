import { getContext, setContext } from 'svelte';
import type { ChartDataDto, ChartSpecDto, ChartTypeDto } from '$lib/api/bindings';
import type { QueryHintSpanDto, TokenSpanDto } from '$lib/api/bindings';
import { analyticsApi, searchApi } from '$lib/api/data';
import { errorText } from '$lib/i18n/errors';
import { ApiError } from '$lib/api/call';
import { unavailable, variants, withSeries, type Variant } from '$lib/charts/variants';
import type { BoolOption } from '$lib/charts/variants';

/** Живое превью обновляется не чаще, чем раз в 150 мс. */
const DEBOUNCE_MS = 150;

export type Preview =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'ready'; data: ChartDataDto }
  | { status: 'error'; message: string };

/** Разбор строки фильтра: по ней рисуются чипы токенов и подсказки. */
export type FilterChips = {
  query: string;
  spans: TokenSpanDto[];
  hints: QueryHintSpanDto[];
};

export type BuilderMode = { kind: 'create' } | { kind: 'edit'; cardId: number };

/** Размер новой карточки по типу графика: ширина в колонках, высота в ячейках по 80 px. */
export function defaultSize(type: ChartTypeDto): { w: number; h: number } {
  switch (type) {
    case 'hbar':
    case 'donut':
    case 'table':
      return { w: 6, h: 5 };
    case 'kpi':
      return { w: 4, h: 2 };
    default:
      return { w: 6, h: 4 };
  }
}

export function newSpec(): ChartSpecDto {
  return {
    version: 1,
    title: 'Новый график',
    type: 'bar',
    metric: 'expenses',
    groupBy: 'month',
    seriesBy: null,
    metrics: [],
    period: { preset: 'ytd' },
    filter: '',
    options: {
      showLimit: false,
      topN: null,
      sort: 'natural',
      cumulative: false,
      comparePrevPeriod: false,
      percent: false
    }
  };
}

/** ViewModel конструктора графика: черновик, превью и доступность значений. */
export class BuilderVm {
  open = $state(false);
  mode = $state.raw<BuilderMode>({ kind: 'create' });
  draft = $state.raw<ChartSpecDto>(newSpec());
  preview = $state.raw<Preview>({ status: 'idle' });
  /** Причина недоступности по id варианта (`type:donut`); `null` — вариант допустим. */
  reasons = $state.raw<Readonly<Record<string, string | null>>>({});
  /** Причина, по которой недопустим сам черновик. */
  problem = $state<string | null>(null);
  saving = $state(false);
  /** Чипы токенов текущего фильтра; `null`, пока фильтр пуст. */
  filterChips = $state.raw<FilterChips | null>(null);

  #timer: ReturnType<typeof setTimeout> | undefined;
  #generation = 0;

  canSave = $derived(this.problem === null && this.preview.status !== 'error');

  beginCreate(): void {
    this.mode = { kind: 'create' };
    this.#show(newSpec());
  }

  beginEdit(cardId: number, spec: ChartSpecDto): void {
    this.mode = { kind: 'edit', cardId };
    this.#show(spec);
  }

  close(): void {
    this.open = false;
    clearTimeout(this.#timer);
    this.#generation += 1;
  }

  #show(spec: ChartSpecDto): void {
    this.draft = spec;
    this.preview = { status: 'loading' };
    this.reasons = {};
    this.problem = null;
    this.filterChips = null;
    this.open = true;
    this.#schedule(0);
  }

  change(patch: Partial<ChartSpecDto>): void {
    this.draft = { ...this.draft, ...patch };
    this.#schedule(DEBOUNCE_MS);
  }

  changeOption(patch: Partial<ChartSpecDto['options']>): void {
    this.change({ options: { ...this.draft.options, ...patch } });
  }

  toggle(option: BoolOption): void {
    this.changeOption({ [option]: !this.draft.options[option] });
  }

  setSeries(series: Parameters<typeof withSeries>[1]): void {
    this.draft = withSeries(this.draft, series);
    this.#schedule(DEBOUNCE_MS);
  }

  /** Показатели серий «по показателям»: от двух до четырёх. */
  toggleMetric(metric: ChartSpecDto['metric']): void {
    const has = this.draft.metrics.includes(metric);
    const metrics = has
      ? this.draft.metrics.filter((m) => m !== metric)
      : [...this.draft.metrics, metric];
    this.change({ metrics, metric: metrics[0] ?? this.draft.metric });
  }

  /** Причина недоступности значения поля или `null`. */
  blocked(id: string): string | null {
    return unavailable(this.reasons, this.problem, id);
  }

  #schedule(delay: number): void {
    clearTimeout(this.#timer);
    const generation = ++this.#generation;
    this.#timer = setTimeout(() => {
      void this.#refresh(generation);
    }, delay);
  }

  async #parseFilter(generation: number, query: string): Promise<void> {
    if (query.trim() === '') {
      this.filterChips = null;
      return;
    }
    try {
      const { spans, hints } = await searchApi.parse(query);
      if (generation === this.#generation) this.filterChips = { query, spans, hints };
    } catch {
      // Чипы — подсказка: ошибка разбора не мешает превью, его причину покажет `analytics_check`.
      if (generation === this.#generation) this.filterChips = null;
    }
  }

  async #refresh(generation: number): Promise<void> {
    const spec = this.draft;
    void this.#parseFilter(generation, spec.filter);
    const all: Variant[] = [{ id: 'self', spec }, ...variants(spec)];
    try {
      const checked = await analyticsApi.check(all.map((v) => v.spec));
      if (generation !== this.#generation) return;
      this.reasons = Object.fromEntries(all.map((v, i) => [v.id, checked[i] ?? null]));
      this.problem = this.reasons['self'] ?? null;
    } catch (e) {
      if (generation !== this.#generation) return;
      this.preview = { status: 'error', message: errorText(e) };
      return;
    }
    if (this.problem !== null) {
      this.preview = { status: 'error', message: this.reasonText(this.problem) };
      return;
    }
    this.preview = { status: 'loading' };
    try {
      const data = await analyticsApi.run(spec);
      if (generation === this.#generation) this.preview = { status: 'ready', data };
    } catch (e) {
      if (generation === this.#generation) {
        this.preview = { status: 'error', message: errorText(e) };
      }
    }
  }

  /** Текст причины для подсказки. */
  reasonText(key: string): string {
    return errorText(new ApiError({ code: 'Validation', messageKey: key, field: null }));
  }
}

const KEY = Symbol('builder-vm');

export function setBuilderVm(vm: BuilderVm): void {
  setContext(KEY, vm);
}

export function getBuilderVm(): BuilderVm {
  return getContext<BuilderVm>(KEY);
}

import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { events, type ChartCardDto, type ChartSpecDto, type DashboardDto } from '$lib/api/bindings';
import { analyticsApi, dashboardsApi } from '$lib/api/data';
import type { CardData } from '$lib/charts/card-data';
import {
  COLUMNS,
  place,
  sameLayout,
  settle,
  SIZES,
  type Box,
  type SizeName
} from '$lib/charts/layout';
import { errorText } from '$lib/i18n/errors';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';

/** Пауза после последнего `data-changed` перед пересчётом всех графиков. */
const REFRESH_DELAY_MS = 300;

const boxOf = (c: ChartCardDto): Box => ({ id: c.id, x: c.x, y: c.y, w: c.w, h: c.h });

/** ViewModel экрана «Аналитика»: дашборды, карточки, раскладка. Числа графиков считает Rust. */
export class AnalyticsVm {
  dashboards = $state.raw<DashboardDto[]>([]);
  activeId = $state<number | null>(null);
  cards = $state.raw<ChartCardDto[]>([]);
  /** Раскладка на экране: во время перетаскивания — предпросмотр, иначе равна сохранённой. */
  layout = $state.raw<Box[]>([]);
  data = new SvelteMap<number, CardData>();
  loading = $state(true);
  error = $state('');

  /** Номер последней загрузки данных: ответы старых запросов отбрасываются. */
  #generation = 0;

  active = $derived(this.dashboards.find((d) => d.id === this.activeId) ?? null);
  card(id: number): ChartCardDto | undefined {
    return this.cards.find((c) => c.id === id);
  }

  async load(): Promise<void> {
    this.loading = true;
    this.error = '';
    try {
      this.dashboards = await dashboardsApi.list();
      const keep = this.dashboards.find((d) => d.id === this.activeId);
      const target = keep ?? this.dashboards.find((d) => d.isDefault) ?? this.dashboards[0];
      this.activeId = target?.id ?? null;
      await this.loadCards();
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.loading = false;
    }
  }

  async select(id: number): Promise<void> {
    if (id === this.activeId) return;
    const previous = this.activeId;
    this.activeId = id;
    try {
      await this.loadCards();
    } catch (e) {
      // Вкладка и карточки должны оставаться согласованными: иначе правки уйдут не на тот дашборд.
      this.activeId = previous;
      toasts.push({ message: errorText(e), kind: 'error' });
    }
  }

  async loadCards(): Promise<void> {
    if (this.activeId === null) {
      this.cards = [];
      this.layout = [];
      return;
    }
    const id = this.activeId;
    const cards = await dashboardsApi.charts(id);
    if (id !== this.activeId) return;
    this.cards = cards;
    this.layout = cards.map(boxOf);
    this.data.clear();
    await this.refreshData();
  }

  /** Пересчитывает данные всех карточек (после изменения записей или правки графика). */
  async refreshData(): Promise<void> {
    const generation = ++this.#generation;
    await Promise.all(
      this.cards.map(async (card) => {
        if (!this.data.has(card.id)) this.data.set(card.id, { status: 'loading' });
        try {
          const data = await analyticsApi.run(card.spec);
          if (generation === this.#generation) this.data.set(card.id, { status: 'ready', data });
        } catch (e) {
          if (generation === this.#generation) {
            this.data.set(card.id, { status: 'error', message: errorText(e) });
          }
        }
      })
    );
  }

  // --- раскладка ---

  /** Предпросмотр при перетаскивании: сохранённое состояние не меняется. */
  preview(id: number, to: Partial<Box>): void {
    const next = place(this.layout, id, to);
    if (!sameLayout(next, this.layout)) this.layout = next;
  }

  cancelPreview(): void {
    this.layout = this.cards.map(boxOf);
  }

  /** Фиксирует положение: карточки прижимаются вверх и раскладка уходит в базу. */
  async commit(id: number, to: Partial<Box>): Promise<void> {
    if (this.activeId === null) return;
    const before = this.cards.map(boxOf);
    const next = settle(before, id, to);
    this.layout = next;
    if (sameLayout(next, before)) return;
    try {
      await dashboardsApi.saveLayout(
        this.activeId,
        next.map((b) => ({ ...b }))
      );
      this.cards = this.cards.map((c) => ({ ...c, ...(next.find((b) => b.id === c.id) ?? {}) }));
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      this.layout = before;
    }
  }

  async resize(id: number, size: SizeName): Promise<void> {
    const w = SIZES.find((s) => s.value === size)?.w;
    if (w !== undefined) await this.commit(id, { w, x: Math.min(this.#x(id), COLUMNS - w) });
  }

  #x(id: number): number {
    return this.card(id)?.x ?? 0;
  }

  // --- карточки ---

  async duplicate(id: number): Promise<void> {
    const card = this.card(id);
    if (card === undefined || this.activeId === null) return;
    try {
      const spec: ChartSpecDto = { ...card.spec, title: `${card.spec.title} (копия)` };
      await dashboardsApi.addChart(this.activeId, spec, card.w, card.h);
      await this.loadCards();
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
    }
  }

  /** Удаление с отменой: карточка возвращается на прежнее место. */
  async remove(id: number): Promise<void> {
    const card = this.card(id);
    const dashboardId = this.activeId;
    if (card === undefined || dashboardId === null) return;
    let currentId = id;
    try {
      await dashboardsApi.removeChart(id);
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      return;
    }
    await this.loadCards();
    const entry = {
      label: `График «${card.spec.title}» удалён`,
      undo: async () => {
        const restored = await dashboardsApi.addChart(dashboardId, card.spec, card.w, card.h);
        currentId = restored.id;
        const boxes = (await dashboardsApi.charts(dashboardId)).map(boxOf);
        await dashboardsApi.saveLayout(
          dashboardId,
          boxes.map((b) => (b.id === restored.id ? { ...b, x: card.x, y: card.y } : { ...b }))
        );
        if (this.activeId === dashboardId) await this.loadCards();
      },
      redo: async () => {
        await dashboardsApi.removeChart(currentId);
        if (this.activeId === dashboardId) await this.loadCards();
      }
    };
    undoStack.record(entry);
    toasts.push({
      message: entry.label,
      action: {
        label: 'Отменить',
        run: () => {
          void undoStack.undo(entry);
        }
      }
    });
  }

  /** Добавляет график из конструктора под остальными. */
  async addChart(spec: ChartSpecDto, w: number, h: number): Promise<boolean> {
    if (this.activeId === null) return false;
    try {
      await dashboardsApi.addChart(this.activeId, spec, w, h);
      await this.loadCards();
      return true;
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      return false;
    }
  }

  /** Сохраняет правку графика; положение карточки не меняется. */
  async updateChart(id: number, spec: ChartSpecDto): Promise<boolean> {
    try {
      const updated = await dashboardsApi.updateChart(id, spec);
      this.cards = this.cards.map((c) => (c.id === id ? updated : c));
      this.data.set(id, { status: 'loading' });
      await this.refreshData();
      return true;
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      return false;
    }
  }

  // --- дашборды ---

  async createDashboard(name: string): Promise<boolean> {
    try {
      const created = await dashboardsApi.create(name);
      this.dashboards = [...this.dashboards, created];
      await this.select(created.id);
      return true;
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      return false;
    }
  }

  async renameDashboard(id: number, name: string): Promise<boolean> {
    try {
      await dashboardsApi.rename(id, name);
      this.dashboards = this.dashboards.map((d) => (d.id === id ? { ...d, name: name.trim() } : d));
      return true;
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      return false;
    }
  }

  async deleteDashboard(id: number): Promise<void> {
    try {
      await dashboardsApi.remove(id);
    } catch (e) {
      toasts.push({ message: errorText(e), kind: 'error' });
      return;
    }
    if (this.activeId === id) this.activeId = null;
    await this.load();
  }

  /** Перерисовывает графики, когда меняются записи. Возвращает cleanup. */
  connect(): () => void {
    let timer: ReturnType<typeof setTimeout> | undefined;
    // Серия правок подряд (быстрое добавление, импорт) пересчитывает графики один раз.
    const off = events.dataChanged.listen(() => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        void this.refreshData();
      }, REFRESH_DELAY_MS);
    });
    return () => {
      clearTimeout(timer);
      void off.then((stop) => {
        stop();
      });
    };
  }
}

const KEY = Symbol('analytics-vm');

export function setAnalyticsVm(vm: AnalyticsVm): void {
  setContext(KEY, vm);
}

export function getAnalyticsVm(): AnalyticsVm {
  return getContext<AnalyticsVm>(KEY);
}

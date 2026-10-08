import type { ChartCardDto, ChartSpecDto, DashboardDto } from '../bindings';
import { mockAnalyticsRun } from './analytics';

/** Моки дашбордов и карточек: состояние в памяти, как у остальных моков. */

type SpecPatch = Partial<ChartSpecDto>;

function baseSpec(title: string, patch: SpecPatch): ChartSpecDto {
  return {
    version: 1,
    title,
    type: 'bar',
    metric: 'spent',
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
    },
    ...patch
  };
}

/** Стандартный дашборд из `core::analytics::standard_dashboard`. */
function standardCards(dashboardId: number, firstId: number): ChartCardDto[] {
  const limitOptions = { ...baseSpec('', {}).options, showLimit: true, sort: 'desc' as const };
  const specs: [ChartSpecDto, number, number, number, number][] = [
    [
      baseSpec('Доходы, расходы и сбережения', {
        metric: 'income',
        seriesBy: 'metric',
        metrics: ['income', 'expenses', 'savings']
      }),
      0,
      0,
      6,
      4
    ],
    [
      baseSpec('Накопления: остаток и сбережения', {
        type: 'line',
        metric: 'free_cum',
        seriesBy: 'metric',
        metrics: ['free_cum', 'savings_cum']
      }),
      6,
      0,
      6,
      4
    ],
    [baseSpec('Траты по статусам', { type: 'stacked_bar', seriesBy: 'status' }), 0, 4, 6, 4],
    [
      baseSpec('Обязательные / желания / сбережения', { type: 'stacked_bar', seriesBy: 'kind' }),
      6,
      4,
      6,
      4
    ],
    [
      baseSpec('Расходы за год по категориям', {
        type: 'hbar',
        metric: 'expenses',
        groupBy: 'category'
      }),
      0,
      8,
      6,
      5
    ],
    [baseSpec('Структура по типам', { type: 'donut', groupBy: 'kind' }), 6, 8, 6, 5],
    [
      baseSpec('Динамика крупных категорий', {
        type: 'line',
        metric: 'expenses',
        seriesBy: 'category',
        filter: 'кат:продукты,одежда,"кафе и доставка","хобби и игры","дом и техника"'
      }),
      0,
      13,
      12,
      4
    ],
    [
      baseSpec('Лимит и факт (текущий месяц)', {
        type: 'hbar',
        metric: 'limit_usage',
        groupBy: 'category',
        period: { preset: 'current_month' },
        options: limitOptions
      }),
      0,
      17,
      12,
      5
    ],
    [baseSpec('Норма сбережений', { type: 'line', metric: 'savings_rate' }), 0, 22, 6, 3]
  ];
  return specs.map(([spec, x, y, w, h], i) => ({
    id: firstId + i,
    dashboardId,
    spec,
    x,
    y,
    w,
    h
  }));
}

let dashboards: DashboardDto[] = [];
let cards: ChartCardDto[] = [];
let nextId = 1;

export function resetMockDashboards(): void {
  nextId = 1;
  dashboards = [{ id: nextId++, name: 'Мой бюджет', isDefault: true }];
  cards = standardCards(1, nextId);
  nextId += cards.length;
}
resetMockDashboards();

/** Хранилище, созданное до: дашбордов нет. */
export function emptyMockDashboards(): void {
  dashboards = [];
  cards = [];
}

function notFound(entity: string, id: number): Promise<never> {
  // IPC отдаёт AppError значением, а не Error
  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
  return Promise.reject({ code: 'NotFound', entity, id });
}

function invalid(messageKey: string): Promise<never> {
  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
  return Promise.reject({ code: 'Validation', messageKey: `errors.${messageKey}`, field: null });
}

type Placement = { id: number; x: number; y: number; w: number; h: number };

/** Упрощённая проверка для моков: настоящие правила — `ChartSpec::validate` в Rust. */
function mockReason(spec: ChartSpecDto): string | null {
  if (spec.filter.includes('источник')) return 'errors.chart.filter_source';
  if (spec.type === 'donut' && (spec.groupBy === 'month' || spec.groupBy === 'none')) {
    return 'errors.chart.donut';
  }
  if (spec.type === 'donut' && spec.seriesBy !== null) return 'errors.chart.donut';
  if (spec.type === 'stacked_bar' && (spec.seriesBy === null || spec.seriesBy === 'metric')) {
    return 'errors.chart.stacked_without_series';
  }
  if (spec.metric === 'savings_rate' && spec.groupBy !== 'month') {
    return 'errors.chart.savings_rate_by_month';
  }
  if (spec.metric === 'limit_usage' && spec.groupBy !== 'category') {
    return 'errors.chart.limit_usage_by_category';
  }
  if (spec.options.showLimit && spec.metric !== 'limit_usage') return 'errors.chart.show_limit';
  if (spec.options.cumulative && spec.groupBy !== 'month') return 'errors.chart.cumulative';
  if (spec.options.comparePrevPeriod && spec.groupBy !== 'month')
    return 'errors.chart.compare_prev';
  return null;
}

export const dashboardHandlers = {
  analytics_check: (args: { specs: ChartSpecDto[] }) => args.specs.map(mockReason),
  analytics_run: (args: { spec: ChartSpecDto }) => mockAnalyticsRun(args.spec),
  analytics_run_many: (args: { specs: ChartSpecDto[] }) =>
    args.specs.map((spec) => {
      const errorKey = mockReason(spec);
      return errorKey === null
        ? { data: mockAnalyticsRun(spec), errorKey: null }
        : { data: null, errorKey };
    }),
  dashboards_list: () => dashboards,
  dashboards_create: (args: { name: string }) => {
    const name = args.name.trim();
    if (name === '') return invalid('dashboard.name_empty');
    const row: DashboardDto = { id: nextId++, name, isDefault: false };
    dashboards = [...dashboards, row];
    return row;
  },
  dashboards_create_default: () => {
    if (dashboards.length > 0) {
      // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
      return Promise.reject({ code: 'Conflict', messageKey: 'errors.dashboard.exists' });
    }
    const row: DashboardDto = { id: nextId++, name: 'Мой бюджет', isDefault: true };
    dashboards = [row];
    cards = standardCards(row.id, nextId);
    nextId += cards.length;
    return row;
  },
  dashboards_rename: (args: { id: number; name: string }) => {
    if (!dashboards.some((d) => d.id === args.id)) return notFound('dashboard', args.id);
    dashboards = dashboards.map((d) => (d.id === args.id ? { ...d, name: args.name.trim() } : d));
    return null;
  },
  dashboards_delete: (args: { id: number }) => {
    if (dashboards.length <= 1) return invalid('dashboard.last');
    const removed = dashboards.find((d) => d.id === args.id);
    if (removed === undefined) return notFound('dashboard', args.id);
    dashboards = dashboards.filter((d) => d.id !== args.id);
    cards = cards.filter((c) => c.dashboardId !== args.id);
    if (removed.isDefault) {
      dashboards = dashboards.map((d, i) => (i === 0 ? { ...d, isDefault: true } : d));
    }
    return null;
  },
  charts_list: (args: { dashboardId: number }) =>
    cards
      .filter((c) => c.dashboardId === args.dashboardId)
      .sort((a, b) => a.y - b.y || a.x - b.x || a.id - b.id),
  charts_create: (args: { dashboardId: number; spec: ChartSpecDto; w: number; h: number }) => {
    if (!dashboards.some((d) => d.id === args.dashboardId)) {
      return notFound('dashboard', args.dashboardId);
    }
    const y = Math.max(
      0,
      ...cards.filter((c) => c.dashboardId === args.dashboardId).map((c) => c.y + c.h)
    );
    const card: ChartCardDto = {
      id: nextId++,
      dashboardId: args.dashboardId,
      spec: args.spec,
      x: 0,
      y,
      w: args.w,
      h: args.h
    };
    cards = [...cards, card];
    return card;
  },
  charts_update: (args: { id: number; spec: ChartSpecDto }) => {
    const card = cards.find((c) => c.id === args.id);
    if (card === undefined) return notFound('chart', args.id);
    const next = { ...card, spec: args.spec };
    cards = cards.map((c) => (c.id === args.id ? next : c));
    return next;
  },
  charts_delete: (args: { id: number }) => {
    if (!cards.some((c) => c.id === args.id)) return notFound('chart', args.id);
    cards = cards.filter((c) => c.id !== args.id);
    return null;
  },
  charts_layout_set: (args: { dashboardId: number; placements: Placement[] }) => {
    cards = cards.map((c) => {
      const p = args.placements.find((q) => q.id === c.id);
      return p !== undefined && c.dashboardId === args.dashboardId
        ? { ...c, x: p.x, y: p.y, w: p.w, h: p.h }
        : c;
    });
    return null;
  }
};

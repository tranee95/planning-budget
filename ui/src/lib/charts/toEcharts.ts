import type { EChartsCoreOption } from 'echarts/core';
import type { ChartDataDto, ChartOptionsDto, ChartSpecDto } from '$lib/api/bindings';
import {
  isForecast,
  isPrevious,
  resolveColor,
  type CategoryColors,
  type CssReader
} from './colors';
import { formatAxis, formatValue } from './format';

/** Единственная точка, где `ChartData` превращается в настройки ECharts. */

export type ChartEnv = {
  css: CssReader;
  categoryColors: CategoryColors;
  reducedMotion: boolean;
};

/** Больше точек — маркеры на линии только мешают. */
const MAX_MARKED_POINTS = 12;
const PREVIOUS_OPACITY = 0.5;

/** Что нужно для отрисовки: тип и параметры. Остальные поля `ChartSpecDto` относятся к запросу. */
export type ChartView = { type: ChartSpecDto['type']; options: ChartOptionsDto };

export const PLAIN_OPTIONS: ChartOptionsDto = {
  showLimit: false,
  topN: null,
  sort: 'natural',
  cumulative: false,
  comparePrevPeriod: false,
  percent: false
};

type Orientation = 'vertical' | 'horizontal';

function orientation(spec: ChartView): Orientation {
  return spec.type === 'hbar' ? 'horizontal' : 'vertical';
}

function seriesOption(
  data: ChartDataDto,
  spec: ChartView,
  env: ChartEnv
): Record<string, unknown>[] {
  const horizontal = orientation(spec) === 'horizontal';
  const stacked = spec.type === 'stacked_bar';
  const isLine = spec.type === 'line' || spec.type === 'area';
  const surface = env.css('--surface');
  const points = data.categories.length;
  const refs = referenceLines(data, env, horizontal);

  return data.series.map((series, index) => {
    const color = resolveColor(series.color, env.css, env.categoryColors);
    const previous = isPrevious(series.color);
    const base: Record<string, unknown> = {
      name: series.name,
      data: series.values,
      emphasis: { focus: 'series' }
    };
    if (isLine) {
      return {
        ...base,
        type: 'line',
        color,
        symbol: 'circle',
        symbolSize: 7,
        showSymbol: points <= MAX_MARKED_POINTS,
        lineStyle: {
          width: 2,
          type: previous || isForecast(series.color) ? 'dashed' : 'solid',
          opacity: previous ? PREVIOUS_OPACITY : 1
        },
        itemStyle: { color, borderColor: surface, borderWidth: 2 },
        areaStyle: spec.type === 'area' ? { color, opacity: 0.14 } : undefined,
        ...(index === 0 ? refs : {})
      };
    }
    return {
      ...base,
      type: 'bar',
      stack: stacked ? 'total' : undefined,
      barMaxWidth: horizontal ? 18 : 32,
      itemStyle: {
        color,
        opacity: previous ? PREVIOUS_OPACITY : 1,
        // 2 px зазор между сегментами и соседними столбцами: поверхность в рамке 1 px
        borderColor: surface,
        borderWidth: stacked ? 1 : 0,
        borderRadius: stacked ? 0 : horizontal ? [0, 4, 4, 0] : [4, 4, 0, 0]
      },
      label:
        horizontal && data.series.length === 1
          ? {
              show: true,
              position: 'right',
              color: env.css('--ink-2'),
              formatter: (p: { value: number | null }) =>
                p.value === null ? '' : formatValue(p.value, data.unit)
            }
          : undefined,
      ...(index === 0 ? refs : {})
    };
  });
}

/** Лимит — штриховая линия, коридор — полоса; рисуются на первой серии. */
function referenceLines(
  data: ChartDataDto,
  env: ChartEnv,
  horizontal: boolean
): { markLine?: unknown; markArea?: unknown } {
  const axis = horizontal ? 'xAxis' : 'yAxis';
  const lines = data.referenceLines.filter((l) => l.to === null);
  const bands = data.referenceLines.filter((l) => l.to !== null);
  const label = { color: env.css('--muted'), fontSize: 12 };
  return {
    markLine:
      lines.length === 0
        ? undefined
        : {
            silent: true,
            symbol: 'none',
            lineStyle: { color: env.css('--muted'), type: 'dashed', width: 1 },
            label: { ...label, formatter: '{b}' },
            data: lines.map((l) => ({ name: l.name, [axis]: l.from }))
          },
    markArea:
      bands.length === 0
        ? undefined
        : {
            silent: true,
            itemStyle: { color: env.css('--accent'), opacity: 0.1 },
            label: { ...label, position: 'insideTopLeft' },
            data: bands.map((b) => [{ name: b.name, [axis]: b.from }, { [axis]: b.to }])
          }
  };
}

function donutOption(data: ChartDataDto, env: ChartEnv): Record<string, unknown> {
  const series = data.series[0];
  return {
    type: 'pie',
    radius: ['58%', '82%'],
    avoidLabelOverlap: true,
    label: { show: false },
    itemStyle: { borderColor: env.css('--surface'), borderWidth: 2, borderRadius: 4 },
    data: data.categories.map((name, i) => ({
      name,
      value: series?.values[i] ?? 0,
      itemStyle: {
        color: resolveColor(data.categoryTokens[i] ?? '', env.css, env.categoryColors)
      }
    }))
  };
}

export function toEcharts(data: ChartDataDto, spec: ChartView, env: ChartEnv): EChartsCoreOption {
  const { css } = env;
  const duration = env.reducedMotion ? 0 : 800;
  const tooltip = {
    confine: true,
    backgroundColor: css('--surface'),
    borderColor: css('--line'),
    textStyle: { color: css('--ink'), fontSize: 13, fontFamily: css('--font-sans') },
    padding: [8, 12],
    extraCssText:
      'box-shadow: var(--shadow); border-radius: 10px; font-variant-numeric: tabular-nums;',
    valueFormatter: (value: unknown) =>
      typeof value === 'number' ? formatValue(value, data.unit) : '—'
  };
  const common = {
    textStyle: { fontFamily: css('--font-sans') },
    animationDuration: duration,
    animationDurationUpdate: env.reducedMotion ? 0 : 450,
    animationEasing: 'quinticOut' as const
  };

  if (spec.type === 'donut') {
    return {
      ...common,
      tooltip: { ...tooltip, trigger: 'item' },
      series: [donutOption(data, env)]
    };
  }

  const horizontal = orientation(spec) === 'horizontal';
  const valueAxis = {
    type: 'value',
    max: spec.options.percent ? 100 : undefined,
    axisLabel: {
      color: css('--muted'),
      fontSize: 12,
      formatter: (v: number) => formatAxis(v, data.unit)
    },
    splitLine: { lineStyle: { color: css('--line-2') } }
  };
  const categoryAxis = {
    type: 'category',
    data: data.categories,
    inverse: horizontal,
    axisTick: { show: false },
    axisLine: { lineStyle: { color: css('--line') } },
    axisLabel: { color: css('--muted'), fontSize: 12, hideOverlap: true }
  };
  const isLine = spec.type === 'line' || spec.type === 'area';
  return {
    ...common,
    grid: { left: 8, right: horizontal ? 56 : 16, top: 16, bottom: 8, containLabel: true },
    tooltip: {
      ...tooltip,
      trigger: 'axis',
      axisPointer: {
        type: isLine ? 'line' : 'shadow',
        lineStyle: { color: css('--line') },
        shadowStyle: { color: css('--ink'), opacity: 0.04 }
      }
    },
    xAxis: horizontal ? valueAxis : categoryAxis,
    yAxis: horizontal ? categoryAxis : valueAxis,
    series: seriesOption(data, spec, env)
  };
}

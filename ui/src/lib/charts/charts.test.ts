import { describe, expect, it } from 'vitest';
import type { ChartDataDto, ChartSpecDto } from '$lib/api/bindings';
import { resolveColor } from './colors';
import { drillQuery } from './drill';
import { formatAxis, formatValue } from './format';
import { toEcharts, type ChartEnv } from './toEcharts';

const vars: Record<string, string> = {
  '--paid-bar': '#paid',
  '--debt-bar': '#debt',
  '--kind-wants': '#wants',
  '--plan-bar': '#plan',
  '--accent': '#accent',
  '--surface': '#surface',
  '--muted': '#muted',
  '--ink': '#ink'
};
const css = (name: string): string => vars[name] ?? '#unknown';
const env: ChartEnv = { css, categoryColors: new Map([[7, '#cat7']]), reducedMotion: false };

function spec(patch: Partial<ChartSpecDto> = {}): ChartSpecDto {
  return {
    version: 1,
    title: 'Тест',
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

function data(patch: Partial<ChartDataDto> = {}): ChartDataDto {
  return {
    categories: ['Январь', 'Февраль'],
    categoryTokens: ['month:2026-01', 'month:2026-02'],
    series: [{ name: 'Траты', color: 'metric.spent', values: [1000, null] }],
    referenceLines: [],
    totals: null,
    unit: 'rub',
    ...patch
  };
}

describe('resolveColor', () => {
  it('maps tokens to theme variables and categories to their stored colour', () => {
    expect(resolveColor('status.paid', css, new Map())).toBe('#paid');
    expect(resolveColor('kind.wants', css, new Map())).toBe('#wants');
    expect(resolveColor('category:7', css, new Map([[7, '#cat7']]))).toBe('#cat7');
    expect(resolveColor('category:8', css, new Map())).toBe('#plan');
    expect(resolveColor('other', css, new Map())).toBe('#plan');
    expect(resolveColor('metric.unknown', css, new Map())).toBe('#accent');
  });

  it('gives the previous-period series the colour of the main one', () => {
    expect(resolveColor('status.paid.prev', css, new Map())).toBe('#paid');
  });
});

describe('formatValue', () => {
  it('formats rubles, percents and counts in ru-RU', () => {
    expect(formatValue(1234.5, 'rub')).toBe('1 235 ₽');
    expect(formatValue(12.6, 'percent')).toBe('12,6 %');
    expect(formatValue(1500, 'count')).toBe('1 500');
  });

  it('shortens axis labels', () => {
    expect(formatAxis(12_000, 'rub')).toBe('12 тыс.');
    expect(formatAxis(1_200_000, 'rub')).toBe('1,2 млн');
    expect(formatAxis(13, 'percent')).toBe('13%');
  });
});

describe('toEcharts', () => {
  it('builds bar series on a category axis with token colours', () => {
    const option = toEcharts(data(), spec(), env);
    expect(option).toMatchObject({
      xAxis: { type: 'category', data: ['Январь', 'Февраль'] },
      series: [{ type: 'bar', itemStyle: { color: '#accent' }, data: [1000, null] }],
      animationDuration: 800
    });
  });

  it('turns animation off for reduced motion', () => {
    const option = toEcharts(data(), spec(), { ...env, reducedMotion: true });
    expect(option).toMatchObject({ animationDuration: 0, animationDurationUpdate: 0 });
  });

  it('stacks segments and separates them with a surface-coloured border', () => {
    const stacked = data({
      series: [
        { name: 'Оплачено', color: 'status.paid', values: [1, 2] },
        { name: 'Долг', color: 'status.debt', values: [3, 4] }
      ]
    });
    const option = toEcharts(stacked, spec({ type: 'stacked_bar', seriesBy: 'status' }), env);
    expect(option).toMatchObject({
      series: [
        { stack: 'total', itemStyle: { borderColor: '#surface', color: '#paid' } },
        { stack: 'total', itemStyle: { color: '#debt' } }
      ]
    });
  });

  it('puts horizontal bars on a reversed category axis with value labels', () => {
    const option = toEcharts(data(), spec({ type: 'hbar' }), env);
    expect(option).toMatchObject({
      yAxis: { type: 'category', inverse: true },
      xAxis: { type: 'value' },
      series: [{ label: { show: true } }]
    });
  });

  it('draws lines with markers only for short series and dashes the previous period', () => {
    const lines = data({
      series: [
        { name: 'Траты', color: 'metric.spent', values: [1, 2] },
        { name: 'Траты (прошлый период)', color: 'metric.spent.prev', values: [3, 4] }
      ]
    });
    const option = toEcharts(lines, spec({ type: 'line' }), env);
    expect(option).toMatchObject({
      series: [{ type: 'line', showSymbol: true }, { lineStyle: { type: 'dashed' } }]
    });
  });

  it('draws the limit as a line and the corridor as a band on the first series', () => {
    const lined = data({
      unit: 'percent',
      referenceLines: [
        { name: 'Лимит', from: 100, to: null },
        { name: 'Коридор', from: 13, to: 15 }
      ]
    });
    const option = toEcharts(lined, spec({ type: 'line', metric: 'savings_rate' }), env);
    expect(option).toMatchObject({
      series: [
        {
          markLine: { data: [{ name: 'Лимит', yAxis: 100 }] },
          markArea: { data: [[{ name: 'Коридор', yAxis: 13 }, { yAxis: 15 }]] }
        }
      ]
    });
  });

  it('builds a donut from the first series with per-sector colours', () => {
    const donut = data({
      categories: ['Обязательные', 'Желания'],
      categoryTokens: ['kind.mandatory', 'kind.wants'],
      series: [{ name: 'Траты', color: 'metric.spent', values: [5, 3] }]
    });
    const option = toEcharts(donut, spec({ type: 'donut', groupBy: 'kind' }), env);
    expect(option).toMatchObject({
      series: [
        {
          type: 'pie',
          data: [
            { name: 'Обязательные', value: 5 },
            { name: 'Желания', value: 3, itemStyle: { color: '#wants' } }
          ]
        }
      ]
    });
    expect(option).not.toHaveProperty('xAxis');
  });

  it('formats tooltip values and shows a dash for missing data', () => {
    const { tooltip } = toEcharts(data(), spec(), env) as {
      tooltip: { valueFormatter: (value: unknown) => string };
    };
    expect(tooltip.valueFormatter(1000)).toBe('1 000 ₽');
    expect(tooltip.valueFormatter(undefined)).toBe('—');
  });
});

describe('drillQuery', () => {
  const now = new Date(2026, 8, 15);

  it('opens the clicked month of a monthly chart', () => {
    expect(drillQuery(spec(), data(), 1, 0, now)).toBe('мес:2026-02');
  });

  it('adds the chart filter, category and the period of the chart', () => {
    const byCategory = data({
      categories: ['Хобби и игры'],
      categoryTokens: ['category:12']
    });
    expect(
      drillQuery(spec({ groupBy: 'category', filter: 'тип:желания' }), byCategory, 0, 0, now)
    ).toBe('тип:желания кат:"Хобби и игры" период:2026-01..2026-09');
  });

  it('adds the series for stacked charts', () => {
    const stacked = data({
      series: [{ name: 'Долг', color: 'status.debt', values: [1, 2] }]
    });
    expect(drillQuery(spec({ seriesBy: 'status' }), stacked, 0, 0, now)).toBe(
      'мес:2026-01 статус:долг'
    );
  });

  it('maps kinds from sectors and uses explicit periods', () => {
    const donut = data({ categories: ['Желания'], categoryTokens: ['kind.wants'] });
    const ranged = spec({
      type: 'donut',
      groupBy: 'kind',
      period: { from: '2026-01', to: '2026-09' }
    });
    expect(drillQuery(ranged, donut, 0, 0, now)).toBe('тип:желания период:2026-01..2026-09');
  });

  it('stops the year at the current month and counts twelve months back', () => {
    const byKind = data({ categories: ['Желания'], categoryTokens: ['kind.wants'] });
    expect(
      drillQuery(spec({ groupBy: 'kind', period: { preset: 'last12' } }), byKind, 0, 0, now)
    ).toBe('тип:желания период:2025-10..2026-09');
    expect(
      drillQuery(spec({ groupBy: 'kind', period: { preset: 'all' } }), byKind, 0, 0, now)
    ).toBe('тип:желания');
  });

  it('limits the list to the kinds the metric counts', () => {
    expect(drillQuery(spec({ metric: 'expenses' }), data(), 0, 0, now)).toBe(
      'мес:2026-01 тип:обязательные,желания,займы'
    );
    expect(drillQuery(spec({ metric: 'savings' }), data(), 0, 0, now)).toBe(
      'мес:2026-01 тип:сбережения'
    );
  });

  it('does not drill into incomes', () => {
    const income = data({ series: [{ name: 'Доходы', color: 'metric.income', values: [1, 2] }] });
    expect(drillQuery(spec({ metric: 'income' }), income, 0, 0, now)).toBeNull();
  });

  it('opens tags by their label', () => {
    const tags = data({ categories: ['#подарки', 'Без тега'], categoryTokens: ['', ''] });
    expect(drillQuery(spec({ groupBy: 'tag', period: { preset: 'all' } }), tags, 0, 0, now)).toBe(
      '#подарки'
    );
    expect(drillQuery(spec({ groupBy: 'tag', period: { preset: 'all' } }), tags, 1, 0, now)).toBe(
      ''
    );
  });
});

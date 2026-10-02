import { expect, test } from 'vitest';
import { mockBonds } from '$lib/api/mock/bonds';
import { factChart, forecastChart, lineSpec } from './bonds-charts';
import { isForecast, resolveColor } from './colors';

test('факт: баланс и внесённое по месяцам года, значения в рублях', () => {
  const dto = mockBonds(2026);
  const chart = factChart(dto.months);
  expect(chart.categories).toHaveLength(12);
  expect(chart.categoryTokens[8]).toBe('month:2026-09');
  const balance = chart.series[0];
  expect(balance?.name).toBe('Баланс');
  expect(balance?.values[8]).toBe((dto.months[8]?.balance ?? 0) / 100);
  expect(chart.unit).toBe('rub');
});

test('прогноз: 60 точек на сценарий от января следующего года, линии помечены как прогноз', () => {
  const dto = mockBonds(2026);
  const chart = forecastChart(2026, dto.scenarios);
  expect(chart.categories).toHaveLength(60);
  expect(chart.categories[0]).toBe('Янв 2027');
  expect(chart.categories[59]).toBe('Дек 2031');
  expect(chart.series.map((s) => s.color)).toEqual([
    'scenario.a.forecast',
    'scenario.b.forecast',
    'scenario.c.forecast'
  ]);
  expect(chart.series.every((s) => isForecast(s.color))).toBe(true);
  const b = dto.scenarios[1];
  expect(chart.series[1]?.values[11]).toBe((b?.y1 ?? 0) / 100);
});

test('цвет сценария берётся из токена без суффикса прогноза', () => {
  const css = (name: string): string => `v(${name})`;
  expect(resolveColor('scenario.b.forecast', css, new Map())).toBe('v(--kind-savings)');
  expect(resolveColor('bonds.balance', css, new Map())).toBe('v(--accent)');
});

test('пустой прогноз не ломает ось', () => {
  expect(forecastChart(2026, []).categories).toEqual([]);
  expect(lineSpec('x').type).toBe('line');
});

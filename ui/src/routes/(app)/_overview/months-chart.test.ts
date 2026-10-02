import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import type { MonthSummaryDto } from '$lib/api/bindings';
import MonthsChart from './MonthsChart.svelte';

function month(patch: Partial<MonthSummaryDto>): MonthSummaryDto {
  return { month: '2026-10', income: 0, expenses: 0, savings: 0, ...patch } as MonthSummaryDto;
}

const props = { range: 'year', year: 2026, onrange: () => undefined } as const;

test('без данных вместо вырожденной сетки показывается заглушка', () => {
  render(MonthsChart, { props: { ...props, months: [month({}), month({ month: '2026-11' })] } });
  expect(screen.getByText('Данных за выбранный период нет.')).toBeTruthy();
  expect(document.querySelector('svg')).toBeNull();
});

test('маленькие суммы не схлопывают подписи оси Y в одинаковые «0»', () => {
  render(MonthsChart, { props: { ...props, months: [month({ income: 150, expenses: 80 })] } });
  const labels = [...document.querySelectorAll('svg text[text-anchor="end"]')].map((t) =>
    t.textContent.trim()
  );
  expect(labels).toHaveLength(5);
  expect(new Set(labels).size).toBe(labels.length);
});

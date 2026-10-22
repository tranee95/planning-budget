import { render, screen, within } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import type { CategoryDto, MonthPlanDto, PlanRowDto } from '$lib/api/bindings';
import PlanCard from './PlanCard.svelte';

const plan = {
  month: '2026-11',
  locked: true,
  income: 10_000_000,
  planExpenses: 3_000_000,
  planSavings: 0,
  planRepayments: 0,
  unallocated: 7_000_000,
  balance: 'unallocated',
  rows: [],
  repayments: [],
  unplanned: 0,
  borrowed: 0,
  saved: 0
} as unknown as MonthPlanDto;

const food = { id: 1, name: 'Продукты' } as CategoryDto;
const row: PlanRowDto = {
  categoryId: 1,
  plan: 3_000_000,
  fact: 3_000_000,
  actual: 0,
  deviation: 0
};

const props = {
  plan,
  busy: false,
  canCopy: false,
  onlock: () => undefined,
  onunlock: () => undefined,
  oncopy: () => undefined,
  onstart: () => undefined,
  month: '2026-09',
  onrepay: () => undefined
};

test('таблица плана: «Фактически» без плановых строк рядом с «Факт + план»', () => {
  render(PlanCard, { props: { ...props, lines: [{ category: food, row }] } });
  const headers = screen.getAllByRole('columnheader').map((h) => h.textContent.trim());
  expect(headers.slice(0, 4)).toEqual(['Статья', 'План', 'Фактически', 'Факт + план']);
  const line = within(screen.getByRole('row', { name: /Продукты/ }));
  const cells = line.getAllByRole('cell').map((c) => c.textContent.replace(/\s/g, ' ').trim());
  expect(cells[0]).toMatch(/^30 000/);
  expect(cells[1]).toMatch(/^0 ₽/);
  expect(cells[2]).toMatch(/^30 000/);
});

test('оплаченная часть попадает в «Фактически»', () => {
  const paid: PlanRowDto = { ...row, actual: 1_200_000 };
  render(PlanCard, { props: { ...props, lines: [{ category: food, row: paid }] } });
  const cells = within(screen.getByRole('row', { name: /Продукты/ }))
    .getAllByRole('cell')
    .map((c) => c.textContent.replace(/\s/g, ' ').trim());
  expect(cells[1]).toMatch(/^12 000/);
});

test('«Отложено» — только оплаченное, запланированное показано отдельно «из плана» (UX-10)', () => {
  const saving = { ...plan, planSavings: 2_124_951, saved: 0 };
  render(PlanCard, { props: { ...props, plan: saving, lines: [] } });
  const term = screen.getByText('Отложено', { selector: 'dt' });
  const value = term.parentElement?.querySelector('dd')?.textContent.replace(/\s/g, ' ').trim();
  expect(value).toMatch(/^0 ₽ из плана 21 250 ₽$/);
});

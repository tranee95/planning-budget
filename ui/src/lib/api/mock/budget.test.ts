import { beforeEach, expect, test } from 'vitest';
import { budgetHandlers, resetMockBudget } from './budget';

beforeEach(() => {
  resetMockBudget();
});

// Формулы мока повторяют core: сверка с границами из Rust.
test('сводка месяца: «в неделю» делится свободное на weeksPerMonth, коридор и добавки целочисленные', () => {
  const { summary } = budgetHandlers.summary_month({ month: '2026-09' });
  expect(summary.perWeek).toBe(Math.round(summary.free / 4));
  const needed = Math.ceil((summary.income * 1300) / 10_000) - summary.savings;
  expect(summary.topUpToMin).toBe(Math.max(0, needed));
  const empty = budgetHandlers.summary_month({ month: '2030-01' }).summary;
  expect(empty.corridor).toBe('noIncome');
  expect(empty.topUpToNorm).toBe(0);
});

test('уровень лимита: меньше 85 % — ok, от 85 % — warn, выше 100 % — over, нулевой факт — ok', () => {
  const { limits } = budgetHandlers.summary_month({ month: '2026-09' });
  for (const r of limits) {
    if (r.limit === null || r.level === null) continue;
    const expected =
      r.fact === 0 ? 'ok' : r.fact > r.limit ? 'over' : r.fact * 100 < r.limit * 85 ? 'ok' : 'warn';
    expect(r.level).toBe(expected);
  }
});

test('итоги лимитов: остаток — лимиты минус расходы, сбережения в сумму не входят', () => {
  const o = budgetHandlers.summary_month({ month: '2026-09' });
  expect(o.limitsRemaining).toBe(o.limitsTotal - o.summary.expenses);
  expect(o.spentVsLimits).toBe(o.limitsTotal === 0 ? null : o.summary.expenses / o.limitsTotal);
  expect(o.limits.find((r) => r.planRateBp !== null)?.level).toBeNull();
});

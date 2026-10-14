import { beforeEach, expect, test } from 'vitest';
import { budgetHandlers, resetMockBudget, setMockToday } from './budget';

beforeEach(() => {
  resetMockBudget();
  // «Сегодня» вне проверяемых месяцев: «На неделю» не зависит от даты запуска тестов.
  setMockToday(new Date(2026, 9, 15));
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

test('автоперенос плана: пустой месяц получает план последнего месяца с данными, один раз', () => {
  const created = budgetHandlers.plan_carry_forward({ month: '2027-03' });
  expect(created).toBeGreaterThan(0);
  const plan = budgetHandlers.plan_month({ month: '2027-03' });
  expect(plan.planExpenses).toBeGreaterThan(0);
  expect(budgetHandlers.plan_carry_forward({ month: '2027-03' })).toBe(0);
});

test('автоперенос плана: месяц с записями и месяц до первых данных не заполняются', () => {
  expect(budgetHandlers.plan_carry_forward({ month: '2026-09' })).toBe(0);
  expect(budgetHandlers.plan_carry_forward({ month: '2000-01' })).toBe(0);
});

test('«На неделю» в текущем месяце: остаток ÷ оставшиеся недели, в прошлом — ÷ 4', () => {
  // Сентябрь, 30 дней; сегодня 15-е: осталось 16 дней = 16/7 недели.
  setMockToday(new Date(2026, 8, 15));
  const sep = budgetHandlers.summary_month({ month: '2026-09' }).summary;
  expect(sep.weeksLeft).toBeCloseTo(16 / 7, 9);
  expect(sep.perWeek).toBe(Math.round((sep.free * 7) / 16));
  const aug = budgetHandlers.summary_month({ month: '2026-08' }).summary;
  expect(aug.weeksLeft).toBeNull();
  expect(aug.perWeek).toBe(Math.round(aug.free / 4));
  // Последний день месяца: один день, остаток × 7.
  setMockToday(new Date(2026, 8, 30));
  const last = budgetHandlers.summary_month({ month: '2026-09' }).summary;
  expect(last.perWeek).toBe(last.free * 7);
});

test('«Свободный остаток по факту»: без записей со статусом «План»', () => {
  const sep = budgetHandlers.summary_month({ month: '2026-09' }).summary;
  const planned = sep.byStatus.planned;
  expect(sep.freeActual).toBe(sep.free + planned);
  const plan = budgetHandlers.plan_month({ month: '2026-09' });
  // «Отложено» не включает плановые записи: те же правила, что на «Сбережениях».
  expect(plan.saved).toBeLessThanOrEqual(sep.savings);
});

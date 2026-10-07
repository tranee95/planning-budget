import type {
  AccumulationDto,
  CategoryDto,
  ChartDataDto,
  ForecastScenarioDto,
  SavingsMonthDto,
  SavingsOverviewDto,
  SavingsParamsDto
} from '../bindings';

/**
 * Мок раздела «Сбережения»: только заполняет поля DTO, формулы живут в Rust.
 * Взнос постоянный: 30 000 ₽ в месяц с января по сентябрь 2026.
 */
const MONTHLY_SAVINGS = 3_000_000;
const MONTHS_WITH_DATA = 9;
const SCENARIOS = [
  { scenario: 'a', name: 'A · цель нормы' },
  { scenario: 'b', name: 'B · норма и экономия' },
  { scenario: 'c', name: 'C · как сейчас' }
] as const;
const SHORT = ['Янв', 'Фев', 'Мар', 'Апр', 'Май', 'Июн', 'Июл', 'Авг', 'Сен', 'Окт', 'Ноя', 'Дек'];

const defaultParams = (): SavingsParamsDto => ({
  annualRateBp: 1600,
  taxBp: 1300,
  initialBalance: 0,
  initialMonth: '2026-01'
});

let params = new Map<number, SavingsParamsDto>();
let fixedPlans = new Map<number, number>();

export function resetMockSavings(): void {
  params = new Map();
  fixedPlans = new Map();
}

/** Возврат к проценту сбрасывает фиксированную сумму (как в хранилище). */
/** Фиксированный план накопления (мастер первого месяца). */
export function setMockFixedPlan(categoryId: number, amount: number): void {
  fixedPlans.set(categoryId, amount);
}

export function clearMockFixedPlan(categoryId: number): void {
  fixedPlans.delete(categoryId);
}

const pad = (n: number): string => String(n).padStart(2, '0');

function scenario(
  kind: (typeof SCENARIOS)[number],
  contribution: number,
  start: number,
  monthly: number
): ForecastScenarioDto {
  let balance = start;
  const balances: number[] = [];
  for (let n = 1; n <= 60; n++) {
    balance = balance * (1 + monthly) + contribution;
    balances.push(Math.round(balance));
  }
  return {
    scenario: kind.scenario,
    name: kind.name,
    contribution,
    y1: balances[11] ?? 0,
    y3: balances[35] ?? 0,
    y5: balances[59] ?? 0,
    coupons60: Math.round(balance - start - contribution * 60),
    balances
  };
}

function forecastChart(year: number, scenarios: readonly ForecastScenarioDto[]): ChartDataDto {
  const categories = Array.from({ length: 60 }, (_, i) => {
    const index = i % 12;
    return `${SHORT[index] ?? ''} ${String(year + 1 + Math.floor(i / 12))}`;
  });
  return {
    categories,
    categoryTokens: categories.map(() => ''),
    series: scenarios.map((s) => ({
      name: s.name,
      color: `scenario.${s.scenario}.forecast`,
      values: s.balances.map((b) => b / 100)
    })),
    referenceLines: [],
    totals: null,
    unit: 'rub'
  };
}

function accumulation(category: CategoryDto, year: number, planRateBp: number): AccumulationDto {
  const p = params.get(category.id) ?? defaultParams();
  const effective = (p.annualRateBp / 10_000) * (1 - p.taxBp / 10_000);
  const monthly = (1 + effective) ** (1 / 12) - 1;
  const fixed = fixedPlans.get(category.id) ?? null;
  let balance = p.initialBalance;
  let deposited = p.initialBalance;
  const months: SavingsMonthDto[] = [];
  for (let m = 1; m <= 12; m++) {
    const key = `${String(year)}-${pad(m)}`;
    if (key < p.initialMonth) {
      months.push({ month: key, savings: 0, coupon: 0, balance: 0, deposited: 0, couponIncome: 0 });
      continue;
    }
    const savings = year === 2026 && m <= MONTHS_WITH_DATA ? MONTHLY_SAVINGS : 0;
    const coupon = balance * monthly;
    balance += savings + coupon;
    deposited += savings;
    months.push({
      month: key,
      savings,
      coupon: Math.round(coupon),
      balance: Math.round(balance),
      deposited,
      couponIncome: Math.round(balance - deposited)
    });
  }
  const plan = fixed ?? Math.round((15_000_000 * planRateBp) / 10_000);
  const decBalance = Math.round(balance);
  const scenarios = [
    scenario(SCENARIOS[0], plan, decBalance, monthly),
    scenario(SCENARIOS[1], plan + 500_000, decBalance, monthly),
    scenario(SCENARIOS[2], MONTHLY_SAVINGS, decBalance, monthly)
  ];
  return {
    categoryId: category.id,
    params: p,
    planKind: fixed === null ? 'percent' : 'fixed',
    planRateBp: fixed === null ? planRateBp : 0,
    planFixed: fixed,
    plan,
    balance: decBalance,
    effectiveRateBp: Math.round(effective * 10_000),
    months,
    scenarios,
    factChart: {
      categories: months.map((m) => SHORT[Number(m.month.slice(5)) - 1] ?? ''),
      categoryTokens: months.map((m) => `month:${m.month}`),
      series: [
        { name: 'Баланс', color: 'bonds.balance', values: months.map((m) => m.balance / 100) },
        { name: 'Внесено', color: 'bonds.deposited', values: months.map((m) => m.deposited / 100) }
      ],
      referenceLines: [],
      totals: null,
      unit: 'rub'
    },
    forecastChart: forecastChart(year, scenarios)
  };
}

export function createSavingsHandlers(
  savingsCategories: () => CategoryDto[],
  planRateBp: (categoryId: number) => number
) {
  const known = (id: number) => savingsCategories().some((c) => c.id === id);
  return {
    savings_overview: (args: { year: number }): SavingsOverviewDto => {
      const items = savingsCategories().map((c) => accumulation(c, args.year, planRateBp(c.id)));
      const total = SCENARIOS.map((kind, i) => {
        const parts = items.map((item) => item.scenarios[i]);
        const sum = (f: (s: ForecastScenarioDto) => number) =>
          parts.reduce((acc, s) => acc + (s ? f(s) : 0), 0);
        return {
          scenario: kind.scenario,
          name: kind.name,
          contribution: sum((s) => s.contribution),
          y1: sum((s) => s.y1),
          y3: sum((s) => s.y3),
          y5: sum((s) => s.y5),
          coupons60: sum((s) => s.coupons60),
          balances: Array.from({ length: 60 }, (_, n) => sum((s) => s.balances[n] ?? 0))
        } satisfies ForecastScenarioDto;
      });
      return {
        year: args.year,
        firstYear: Math.min(args.year, 2026),
        items,
        totalBalance: items.reduce((s, i) => s + i.balance, 0),
        monthPlan: items.reduce((s, i) => s + i.plan, 0),
        totalScenarios: total,
        totalForecastChart: forecastChart(args.year, total)
      };
    },
    savings_params_set: (args: { categoryId: number; params: SavingsParamsDto }) => {
      if (!known(args.categoryId)) {
        // IPC отдаёт AppError значением, а не Error
        // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
        return Promise.reject({
          code: 'Validation',
          messageKey: 'errors.savings.not_a_savings_category',
          field: null
        });
      }
      params.set(args.categoryId, args.params);
      return null;
    },
    savings_fixed_set: (args: { categoryId: number; amount: number }) => {
      if (!known(args.categoryId)) {
        // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
        return Promise.reject({
          code: 'Validation',
          messageKey: 'errors.savings.not_a_savings_category',
          field: null
        });
      }
      fixedPlans.set(args.categoryId, args.amount);
      return null;
    }
  };
}

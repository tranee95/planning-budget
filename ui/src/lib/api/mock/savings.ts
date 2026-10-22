import type { CategoryDto, SavingsProgressDto } from '../bindings';

/**
 * Мок раздела «Сбережения»: только заполняет поля DTO, формулы живут в Rust.
 * Взнос постоянный: 30 000 ₽ в месяц с января по сентябрь 2026.
 */
const MONTHLY_SAVINGS = 3_000_000;
const MONTHS_WITH_DATA = 9;
const SHORT = ['Янв', 'Фев', 'Мар', 'Апр', 'Май', 'Июн', 'Июл', 'Авг', 'Сен', 'Окт', 'Ноя', 'Дек'];

let fixedPlans = new Map<number, number>();
/** Суммы накоплений за месяцы (обычные оплаченные траты в категориях-сбережениях). */
export type SavingsFact = { categoryId: number; month: string; amount: number };
let savingsFacts: () => SavingsFact[] = () => [];

export function resetMockSavings(): void {
  fixedPlans = new Map();
}

/** Возврат к проценту сбрасывает фиксированную сумму (как в хранилище). */
/** Фиксированный план накопления (мастер первого месяца). */
export function setMockFixedPlan(categoryId: number, amount: number): void {
  fixedPlans.set(categoryId, amount);
}

export const mockFixedPlan = (categoryId: number): number | null =>
  fixedPlans.get(categoryId) ?? null;

export function clearMockFixedPlan(categoryId: number): void {
  fixedPlans.delete(categoryId);
}

const pad = (n: number): string => String(n).padStart(2, '0');

/** Взносы категории по месяцам года: постоянный взнос января–сентября 2026 плюс введённые суммы. */
function monthlySavings(category: CategoryDto, year: number): { month: string; savings: number }[] {
  const months: { month: string; savings: number }[] = [];
  for (let m = 1; m <= 12; m++) {
    const key = `${String(year)}-${pad(m)}`;
    const entered = savingsFacts()
      .filter((f) => f.categoryId === category.id && f.month === key)
      .reduce((sum, f) => sum + f.amount, 0);
    months.push({
      month: key,
      savings: (year === 2026 && m <= MONTHS_WITH_DATA ? MONTHLY_SAVINGS : 0) + entered
    });
  }
  return months;
}

function nextMonth(month: string): string {
  const y = Number(month.slice(0, 4));
  const m = Number(month.slice(5));
  return m === 12 ? `${String(y + 1)}-01` : `${String(y)}-${String(m + 1).padStart(2, '0')}`;
}

export function createSavingsHandlers(
  savingsCategories: () => CategoryDto[],
  facts: () => SavingsFact[],
  allSavingsCategories: () => CategoryDto[]
) {
  savingsFacts = facts;
  const known = (id: number) => savingsCategories().some((c) => c.id === id);
  return {
    savings_progress: (args: { month: string }): SavingsProgressDto => {
      const year = Number(args.month.slice(0, 4));
      // Все накопления, в том числе закрытые цели: их суммы остаются в итоге и на графике.
      const perCategory = new Map<number, Map<string, number>>();
      const perMonth = new Map<string, number>();
      for (const c of allSavingsCategories()) {
        const own = new Map<string, number>();
        for (const m of monthlySavings(c, year)) {
          if (m.month > args.month || m.savings === 0) continue;
          own.set(m.month, m.savings);
          perMonth.set(m.month, (perMonth.get(m.month) ?? 0) + m.savings);
        }
        if (own.size > 0) perCategory.set(c.id, own);
      }
      const keys = [...perMonth.keys()].sort();
      const first = keys.find((k) => (perMonth.get(k) ?? 0) > 0);
      const months: SavingsProgressDto['months'] = [];
      let total = 0;
      // как в ядре: месяцы подряд от первого взноса до текущего, пропуски — нулями
      const run: string[] = [];
      for (let m = first; m !== undefined && m <= args.month; m = nextMonth(m)) run.push(m);
      for (const month of run) {
        const deposited = perMonth.get(month) ?? 0;
        total += deposited;
        months.push({ month, deposited, cumulative: total });
      }
      const included = allSavingsCategories().filter((c) => perCategory.has(c.id));
      return {
        months,
        total,
        categories: included.map((c) => ({
          categoryId: c.id,
          name: c.name,
          color: c.color,
          archived: c.archived,
          total: months.reduce((sum, m) => sum + (perCategory.get(c.id)?.get(m.month) ?? 0), 0)
        })),
        chart: {
          categories: months.map(
            (m) => `${SHORT[Number(m.month.slice(5)) - 1] ?? ''} ${m.month.slice(0, 4)}`
          ),
          categoryTokens: months.map((m) => `month:${m.month}`),
          series: included.map((c) => ({
            name: c.name,
            color: `category:${String(c.id)}`,
            values: months.map((m) => (perCategory.get(c.id)?.get(m.month) ?? 0) / 100)
          })),
          referenceLines: [],
          totals: null,
          unit: 'rub'
        }
      };
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

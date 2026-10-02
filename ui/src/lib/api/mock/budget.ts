import type {
  AppError,
  CategoryDto,
  CategoryInput,
  CategoryYearRowDto,
  CategoryPatchDto_Deserialize,
  LimitEntryDto,
  SavingsRateDto,
  SavedFilterDto,
  TagDto,
  IncomeDto,
  IncomeInput,
  IncomePatchDto_Deserialize,
  LimitRowDto,
  MonthOverviewDto,
  SettingsDto,
  SettingsPatchDto,
  YearSummaryDto,
  StatusAmountsDto,
  TransactionDto,
  TransactionInput,
  TransactionPatchDto_Deserialize,
  TxStatusDto
} from '../bindings';
import { mockIncomeList, mockSearch, mockTransactionList } from './search';

/**
 * Моки команд данных: небольшой набор за сентябрь 2026 (суммы в копейках).
 * Сводка здесь считается упрощённо: настоящие формулы живут в Rust (`core::calc`),
 * мок нужен только чтобы экраны и ViewModel получали правдоподобные DTO.
 */

const MONTH = '2026-09';

const seedCategories = (): CategoryDto[] => [
  {
    id: 1,
    name: 'Продукты',
    kind: 'mandatory',
    color: '#4f9d69',
    sortOrder: 1,
    note: null,
    archived: false
  },
  {
    id: 2,
    name: 'Жильё',
    kind: 'mandatory',
    color: '#5b7fd6',
    sortOrder: 2,
    note: null,
    archived: false
  },
  {
    id: 3,
    name: 'Развлечения',
    kind: 'wants',
    color: '#d6893a',
    sortOrder: 3,
    note: null,
    archived: false
  },
  {
    id: 4,
    name: 'Накопления',
    kind: 'savings',
    color: '#8a6fd1',
    sortOrder: 4,
    note: null,
    archived: false
  }
];

let categories = seedCategories();

/** История лимитов: запись действует с `validFrom` до следующей. */
const limitHistory = new Map<number, LimitEntryDto[]>();
const seedLimits = (): void => {
  limitHistory.clear();
  limitHistory.set(1, [{ validFrom: '2026-01', amount: 3_000_000 }]);
  limitHistory.set(2, [{ validFrom: '2026-01', amount: 4_500_000 }]);
  limitHistory.set(3, [{ validFrom: '2026-01', amount: 1_000_000 }]);
};
seedLimits();
let savingsRates: SavingsRateDto[] = [{ categoryId: 4, validFrom: '2026-01', rateBp: 1400 }];

/** Процент плана месяца: у каждой категории действует строка с последней `validFrom` не позже месяца. */
function planRateBp(month: string): number {
  const latest = new Map<number, SavingsRateDto>();
  for (const r of savingsRates) {
    const known = latest.get(r.categoryId);
    if (r.validFrom <= month && (!known || known.validFrom < r.validFrom))
      latest.set(r.categoryId, r);
  }
  return [...latest.values()].reduce((sum, r) => sum + r.rateBp, 0);
}

function limitFor(categoryId: number, month: string): number | null {
  const entries = (limitHistory.get(categoryId) ?? []).filter((e) => e.validFrom <= month);
  return entries.sort((a, b) => a.validFrom.localeCompare(b.validFrom)).at(-1)?.amount ?? null;
}

function seedTransactions(): TransactionDto[] {
  const tx = (
    id: number,
    categoryId: number,
    title: string,
    amount: number,
    status: TxStatusDto,
    month = MONTH,
    tagIds: number[] = []
  ): TransactionDto => ({
    id,
    month,
    date: null,
    categoryId,
    title,
    amount,
    status,
    comment: null,
    tagIds
  });
  return [
    tx(1, 1, 'Магазин у дома', 1_250_000, 'paid'),
    tx(2, 1, 'Рынок', 480_000, 'unplanned'),
    tx(3, 2, 'Аренда', 4_200_000, 'paid'),
    tx(4, 2, 'Интернет', 90_000, 'planned'),
    tx(5, 3, 'Кино', 140_000, 'debt'),
    tx(6, 4, 'Вклад', 2_000_000, 'planned'),
    // История «Ленты» для поиска: семь оплаченных трат на 90 681 ₽ и одна плановая.
    ...[980_000, 1_145_000, 1_230_000, 1_313_100, 1_420_000, 1_560_000, 1_420_000].map(
      (amount, i) => tx(11 + i, 1, 'Лента', amount, 'paid', `2024-0${String(i + 1)}`)
    ),
    tx(18, 1, 'Лента', 500_000, 'planned', '2024-08', [1])
  ];
}

function seedIncomes(): IncomeDto[] {
  return [
    {
      id: 1,
      month: MONTH,
      date: null,
      sourceName: 'Зарплата',
      amount: 9_000_000,
      status: 'received',
      comment: null
    },
    {
      id: 2,
      month: MONTH,
      date: null,
      sourceName: 'Подработка',
      amount: 1_500_000,
      status: 'expected',
      comment: null
    }
  ];
}

let transactions = seedTransactions();
let incomes = seedIncomes();
// Удалённые строки хранятся отдельно: tx_restore возвращает их по id.
let deletedTx = new Map<number, TransactionDto>();
let deletedIncomes = new Map<number, IncomeDto>();
let nextId = 100;
const seedTags = (): TagDto[] => [
  { id: 1, name: 'Подарки' },
  { id: 2, name: 'Работа' }
];
let tags = seedTags();
let savedFilters: SavedFilterDto[] = [];

/** Возвращает мок данных к исходному набору (для тестов). */
export function resetMockBudget(): void {
  transactions = seedTransactions();
  incomes = seedIncomes();
  deletedTx = new Map();
  deletedIncomes = new Map();
  nextId = 100;
  tags = seedTags();
  savedFilters = [];
  categories = seedCategories();
  seedLimits();
  savingsRates = [{ categoryId: 4, validFrom: '2026-01', rateBp: 1400 }];
}

function setArchived(id: number, archived: boolean): Promise<never> | null {
  if (!categories.some((c) => c.id === id)) return notFound('category', id);
  categories = categories.map((c) => (c.id === id ? { ...c, archived } : c));
  return null;
}

function notFound(entity: string, id: number): Promise<never> {
  const error: AppError = { code: 'NotFound', entity, id };
  // IPC отдаёт AppError значением, а не Error
  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
  return Promise.reject(error);
}

/** Применяет патч: `undefined` — поле не меняется, `null` — значение (как в Rust-патче). */
function applyPatch<T extends object>(row: T, patch: Partial<Record<keyof T, unknown>>): T {
  const next = { ...row };
  for (const key of Object.keys(patch) as (keyof T)[]) {
    const value = patch[key];
    if (value === undefined) continue;
    // Пустое значение очищает только необязательные поля (`date`, `comment`).
    if (value === null && !(key === 'date' || key === 'comment')) continue;
    next[key] = value as T[keyof T];
  }
  return next;
}

function emptyStatuses(): StatusAmountsDto {
  return { paid: 0, debt: 0, unplanned: 0, planned: 0 };
}

function overview(month: string): MonthOverviewDto {
  const rows = transactions.filter((t) => t.month === month);
  const byStatus = emptyStatuses();
  const byKind = { mandatory: 0, wants: 0, savings: 0, loans: 0 };
  const limitRows: LimitRowDto[] = [];
  for (const category of categories) {
    const own = rows.filter((t) => t.categoryId === category.id);
    const statuses = emptyStatuses();
    for (const t of own) statuses[t.status] += t.amount;
    const fact = own.reduce((sum, t) => sum + t.amount, 0);
    for (const status of Object.keys(statuses) as TxStatusDto[])
      byStatus[status] += statuses[status];
    byKind[category.kind] += fact;
    const limit = limitFor(category.id, month);
    limitRows.push({
      categoryId: category.id,
      fact,
      limit,
      remaining: limit === null ? null : limit - fact,
      usage: limit === null ? null : fact / limit,
      level: limit === null ? null : fact > limit ? 'over' : fact / limit >= 0.9 ? 'warn' : 'ok',
      byStatus: statuses
    });
  }
  const expenses = rows
    .filter((t) => categories.find((c) => c.id === t.categoryId)?.kind !== 'savings')
    .reduce((sum, t) => sum + t.amount, 0);
  const own = incomes.filter((i) => i.month === month);
  const income = own.reduce((sum, i) => sum + i.amount, 0);
  const incomeReceived = own
    .filter((i) => i.status === 'received')
    .reduce((s, i) => s + i.amount, 0);
  const savings = byKind.savings;
  const limitsTotal = limitRows.reduce((sum, r) => sum + (r.limit ?? 0), 0);
  return {
    summary: {
      month,
      income,
      incomeReceived,
      incomeExpected: income - incomeReceived,
      expenses,
      savings,
      free: income - expenses - savings,
      freeCum: income - expenses - savings,
      savingsCum: savings,
      savingsRate: income === 0 ? null : savings / income,
      unspentRate: null,
      savingsPlanRateBp: planRateBp(month),
      savingsPlan: Math.round(income * 0.14),
      savingsGap: savings - Math.round(income * 0.14),
      perWeek: Math.round(expenses / 4.33),
      corridor: income === 0 ? 'noIncome' : 'within',
      topUpToMin: 0,
      topUpToNorm: 0,
      byStatus,
      byKind
    },
    limits: limitRows,
    limitsTotal,
    limitsRemaining:
      limitsTotal - limitRows.reduce((sum, r) => sum + (r.limit === null ? 0 : r.fact), 0),
    spentVsLimits: null
  };
}

/** Текущие настройки мока: от них считает мок облигаций. */
export const mockSettings = (): SettingsDto => settings;

let settings: SettingsDto = {
  savingsMinBp: 1300,
  savingsNormBp: 1400,
  savingsMaxBp: 1500,
  bondsRateBp: 1600,
  bondsCouponTaxBp: 1300,
  bondsInitialBalance: 0,
  bondsInitialMonth: '2026-01',
  weeksPerMonth: 4,
  autolockMinutes: 15,
  lockOnMinimize: false,
  backupAutoDaily: false
};

function yearSummary(year: number): YearSummaryDto {
  const months = Array.from(
    { length: 12 },
    (_, i) => overview(`${String(year)}-${String(i + 1).padStart(2, '0')}`).summary
  );
  type Row = (typeof months)[number];
  const filled = months.filter((m) => m.income > 0 || m.expenses > 0);
  const sum = (pick: (m: Row) => number) => months.reduce((total, m) => total + pick(m), 0);
  const avg = (pick: (m: Row) => number) =>
    filled.length === 0 ? 0 : Math.round(sum(pick) / filled.length);
  const income = sum((m) => m.income);
  return {
    year,
    months,
    income,
    expenses: sum((m) => m.expenses),
    savings: sum((m) => m.savings),
    free: sum((m) => m.free),
    monthsWithData: filled.length,
    avgIncome: avg((m) => m.income),
    avgExpenses: avg((m) => m.expenses),
    avgSavings: avg((m) => m.savings),
    avgFree: avg((m) => m.free),
    savingsRate: income === 0 ? null : sum((m) => m.savings) / income,
    byStatus: emptyStatuses(),
    byKind: { mandatory: 0, wants: 0, savings: 0, loans: 0 },
    categories: categoryYearRows(year),
    balance: {
      avgIncome: avg((m) => m.income),
      savingsTarget: Math.round(avg((m) => m.income) * 0.14),
      limitsSum: categoryYearRows(year).reduce((sum, c) => sum + (c.limitNow ?? 0), 0),
      buffer: 0,
      bufferRate: null,
      economy: 0
    }
  };
}

function categoryYearRows(year: number): CategoryYearRowDto[] {
  const month = `${String(year)}-12`;
  return categories.map((c) => {
    const rows = transactions.filter(
      (t) => t.categoryId === c.id && t.month.startsWith(String(year))
    );
    const total = rows.reduce((sum, t) => sum + t.amount, 0);
    const months = new Set(rows.map((t) => t.month)).size;
    const avg = months === 0 ? 0 : Math.round(total / months);
    const limitNow = limitFor(c.id, month);
    return {
      categoryId: c.id,
      total,
      avg,
      limitNow,
      avgMinusLimit: limitNow === null ? null : avg - limitNow,
      shareInExpenses: null
    };
  });
}

const categoryNotFound = (id: number): Promise<never> => notFound('category', id);

export const budgetHandlers = {
  categories_create: (args: { input: CategoryInput }) => {
    const row: CategoryDto = {
      ...args.input,
      id: nextId++,
      sortOrder: categories.length + 1,
      archived: false
    };
    categories = [...categories, row];
    return row;
  },
  categories_update: (args: { id: number; patch: CategoryPatchDto_Deserialize }) => {
    const previous = categories.find((c) => c.id === args.id);
    if (!previous) return categoryNotFound(args.id);
    const { name, color, note } = args.patch;
    const current: CategoryDto = {
      ...previous,
      name: name ?? previous.name,
      color: color ?? previous.color,
      note: note === undefined ? previous.note : note
    };
    categories = categories.map((c) => (c.id === args.id ? current : c));
    return current;
  },
  categories_archive: (args: { id: number }) => setArchived(args.id, true),
  categories_unarchive: (args: { id: number }) => setArchived(args.id, false),
  categories_delete: (args: { id: number }) => {
    if (!categories.some((c) => c.id === args.id)) return categoryNotFound(args.id);
    categories = categories.filter((c) => c.id !== args.id);
    return null;
  },
  categories_reorder: (args: { ids: number[] }) => {
    const byId = new Map(categories.map((c) => [c.id, c]));
    categories = args.ids.flatMap((id, i) => {
      const c = byId.get(id);
      return c ? [{ ...c, sortOrder: i + 1 }] : [];
    });
    return categories;
  },
  limits_set: (args: { categoryId: number; validFrom: string; amount: number }) => {
    const rest = (limitHistory.get(args.categoryId) ?? []).filter(
      (e) => e.validFrom !== args.validFrom
    );
    limitHistory.set(args.categoryId, [
      ...rest,
      { validFrom: args.validFrom, amount: args.amount }
    ]);
    return null;
  },
  limits_clear: (args: { categoryId: number; validFrom: string }) => {
    const rest = (limitHistory.get(args.categoryId) ?? []).filter(
      (e) => e.validFrom !== args.validFrom
    );
    limitHistory.set(args.categoryId, rest);
    return null;
  },
  limits_history: (args: { categoryId: number }) =>
    [...(limitHistory.get(args.categoryId) ?? [])].sort((a, b) =>
      a.validFrom.localeCompare(b.validFrom)
    ),
  savings_rates_list: () => savingsRates,
  savings_rate_set: (args: { categoryId: number; validFrom: string; rateBp: number }) => {
    savingsRates = [
      ...savingsRates.filter(
        (r) => !(r.categoryId === args.categoryId && r.validFrom === args.validFrom)
      ),
      { categoryId: args.categoryId, validFrom: args.validFrom, rateBp: args.rateBp }
    ];
    return null;
  },
  savings_override_set: () => null,
  savings_override_clear: () => null,

  settings_get: () => settings,
  settings_set: (args: { patch: SettingsPatchDto }) => {
    const defined = Object.fromEntries(Object.entries(args.patch).filter(([, v]) => v != null));
    settings = { ...settings, ...defined };
    return settings;
  },
  summary_year: (args: { year: number }) => yearSummary(args.year),

  categories_list: (args: { includeArchived: boolean }) =>
    categories.filter((c) => args.includeArchived || !c.archived),

  summary_month: (args: { month: string }) => overview(args.month),

  search: (args: { query: string; requestId: number }) =>
    mockSearch(args.query, args.requestId, { transactions, incomes, categories, tags }),

  tx_list: (args: { month: string }) => transactions.filter((t) => t.month === args.month),
  tx_create: (args: { input: TransactionInput }) => {
    const row: TransactionDto = { ...args.input, id: nextId++, tagIds: [] };
    transactions = [...transactions, row];
    return row;
  },
  tx_update: (args: { id: number; patch: TransactionPatchDto_Deserialize }) => {
    const previous = transactions.find((t) => t.id === args.id);
    if (!previous) return notFound('transaction', args.id);
    const current = applyPatch(previous, args.patch);
    transactions = transactions.map((t) => (t.id === args.id ? current : t));
    return { current, previous };
  },
  tx_set_status: (args: { id: number; status: TxStatusDto }) => {
    const previous = transactions.find((t) => t.id === args.id);
    if (!previous) return notFound('transaction', args.id);
    const current = { ...previous, status: args.status };
    transactions = transactions.map((t) => (t.id === args.id ? current : t));
    return { current, previous };
  },
  tx_delete: (args: { id: number }) => {
    const row = transactions.find((t) => t.id === args.id);
    if (!row) return notFound('transaction', args.id);
    transactions = transactions.filter((t) => t.id !== args.id);
    deletedTx.set(row.id, row);
    return row;
  },
  tx_restore: (args: { id: number }) => {
    const row = deletedTx.get(args.id);
    if (!row) return notFound('transaction', args.id);
    deletedTx.delete(args.id);
    transactions = [...transactions, row];
    return row;
  },

  incomes_list: (args: { month: string }) => incomes.filter((i) => i.month === args.month),
  incomes_create: (args: { input: IncomeInput }) => {
    const row: IncomeDto = { ...args.input, id: nextId++ };
    incomes = [...incomes, row];
    return row;
  },
  incomes_update: (args: { id: number; patch: IncomePatchDto_Deserialize }) => {
    const previous = incomes.find((i) => i.id === args.id);
    if (!previous) return notFound('income', args.id);
    const current = applyPatch(previous, args.patch);
    incomes = incomes.map((i) => (i.id === args.id ? current : i));
    return { current, previous };
  },
  incomes_delete: (args: { id: number }) => {
    const row = incomes.find((i) => i.id === args.id);
    if (!row) return notFound('income', args.id);
    incomes = incomes.filter((i) => i.id !== args.id);
    deletedIncomes.set(row.id, row);
    return row;
  },
  incomes_restore: (args: { id: number }) => {
    const row = deletedIncomes.get(args.id);
    if (!row) return notFound('income', args.id);
    deletedIncomes.delete(args.id);
    incomes = [...incomes, row];
    return row;
  },

  tx_search: (args: { query: string; requestId: number }) =>
    mockTransactionList(args.query, args.requestId, { transactions, incomes, categories, tags }),
  incomes_search: (args: { query: string; requestId: number }) =>
    mockIncomeList(args.query, args.requestId, { transactions, incomes, categories, tags }),

  tags_list: () => tags,
  tags_create: (args: { name: string }) => {
    const row: TagDto = { id: nextId++, name: args.name.trim() };
    tags = [...tags, row];
    return row;
  },
  tx_tags_set: (args: { id: number; tagIds: number[] }) => {
    if (!transactions.some((t) => t.id === args.id)) return notFound('transaction', args.id);
    transactions = transactions.map((t) => (t.id === args.id ? { ...t, tagIds: args.tagIds } : t));
    return null;
  },

  filters_list: () => [...savedFilters].sort((a, b) => a.name.localeCompare(b.name, 'ru')),
  filters_save: (args: { name: string; query: string }) => {
    const name = args.name.trim();
    const same = savedFilters.find((f) => f.name.toLowerCase() === name.toLowerCase());
    const row: SavedFilterDto = { id: same?.id ?? nextId++, name, query: args.query.trim() };
    savedFilters = [...savedFilters.filter((f) => f.id !== row.id), row];
    return row;
  },
  filters_delete: (args: { id: number }) => {
    savedFilters = savedFilters.filter((f) => f.id !== args.id);
    return null;
  },

  tx_suggest: (args: { query: string }) => {
    const words = args.query.toLowerCase().split(/s+/).filter(Boolean);
    if (words.length === 0) return [];
    const found = new Map<string, { title: string; categoryId: number; uses: number }>();
    for (const t of transactions) {
      const title = t.title.toLowerCase();
      if (title === args.query.trim().toLowerCase()) continue;
      if (!words.every((w) => title.split(/s+/).some((h) => h.startsWith(w)))) continue;
      const entry = found.get(title);
      found.set(title, { title: t.title, categoryId: t.categoryId, uses: (entry?.uses ?? 0) + 1 });
    }
    return [...found.values()].sort((a, b) => b.uses - a.uses).slice(0, 5);
  },
  tx_category_usage: () => {
    const used = new Map<number, number>();
    for (const t of transactions) used.set(t.categoryId, (used.get(t.categoryId) ?? 0) + 1);
    return [...used]
      .map(([categoryId, uses]) => ({ categoryId, uses }))
      .sort((a, b) => b.uses - a.uses);
  }
};

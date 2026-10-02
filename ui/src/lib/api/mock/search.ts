import type {
  CategoryDto,
  IncomeDto,
  IncomeSearchDto,
  QueryHintSpanDto,
  SearchResultDto,
  TagDto,
  TokenSpanDto,
  TransactionDto,
  TransactionSearchDto,
  TxStatusDto
} from '../bindings';

/**
 * Упрощённый поиск для моков: слова (префикс по названию и категории), `статус:`, `сумма>N` / `сумма<N`,
 * `#тег`, `период:ГГГГ-ММ` или `ГГГГ-ММ..ГГГГ-ММ`. Полный язык запроса и ранжирование живут в Rust
 * (`core::query`, `storage::search`).
 */

const STATUS_WORDS: Record<string, TxStatusDto> = {
  оплачено: 'paid',
  долг: 'debt',
  внеплан: 'unplanned',
  план: 'planned'
};

const norm = (s: string): string => s.toLowerCase().replaceAll('ё', 'е');

interface MockData {
  transactions: TransactionDto[];
  incomes: IncomeDto[];
  categories: CategoryDto[];
  tags: TagDto[];
}

interface Parsed {
  spans: TokenSpanDto[];
  hints: QueryHintSpanDto[];
  words: string[];
  status: TxStatusDto | null;
  tag: string | null;
  from: string;
  to: string;
  min: number;
  max: number;
}

function parseQuery(query: string): Parsed {
  const parsed: Parsed = {
    spans: [],
    hints: [],
    words: [],
    status: null,
    tag: null,
    from: '',
    to: '￿',
    min: 0,
    max: Number.POSITIVE_INFINITY
  };
  let at = 0;
  for (const part of query.split(/(\s+)/)) {
    const len = Array.from(part).length;
    const start = at;
    at += len;
    if (part.trim() === '') continue;
    const token = norm(part);
    const amount = /^сумма([<>])(\d+)$/.exec(token);
    const period = /^период:(\d{4}-\d{2})(?:\.\.(\d{4}-\d{2}))?$/.exec(token);
    let kind: TokenSpanDto['kind'] = 'text';
    if (token.startsWith('статус:') && token.slice(7) in STATUS_WORDS) {
      parsed.status = STATUS_WORDS[token.slice(7)] ?? null;
      kind = 'status';
    } else if (token.startsWith('статус:')) {
      parsed.hints.push({ start, end: start + len, hint: 'unknown_value' });
      parsed.words.push(token);
    } else if (amount) {
      const kopecks = Number(amount[2]) * 100;
      if (amount[1] === '>') parsed.min = kopecks + 1;
      else parsed.max = kopecks - 1;
      kind = 'amount';
    } else if (period) {
      parsed.from = period[1] ?? '';
      parsed.to = period[2] ?? period[1] ?? '';
      kind = 'month';
    } else if (token.startsWith('#') && token.length > 1) {
      parsed.tag = token.slice(1);
      kind = 'tag';
    } else {
      parsed.words.push(token);
    }
    parsed.spans.push({ start, end: start + len, kind, negated: false });
  }
  return parsed;
}

const sum = (items: { amount: number }[]): number => items.reduce((s, i) => s + i.amount, 0);

function matchTransactions(parsed: Parsed, data: MockData): TransactionDto[] {
  const names = new Map(data.categories.map((c) => [c.id, c.name]));
  const tagIds = new Set(
    data.tags.filter((t) => parsed.tag !== null && norm(t.name) === parsed.tag).map((t) => t.id)
  );
  const matches = (haystack: string): boolean =>
    parsed.words.every((w) =>
      norm(haystack)
        .split(/\s+/)
        .some((h) => h.startsWith(w))
    );
  return data.transactions
    .filter(
      (t) =>
        matches(`${t.title} ${names.get(t.categoryId) ?? ''}`) &&
        (parsed.status === null || t.status === parsed.status) &&
        (parsed.tag === null || t.tagIds.some((id) => tagIds.has(id))) &&
        t.amount >= parsed.min &&
        t.amount <= parsed.max &&
        t.month >= parsed.from &&
        t.month <= parsed.to
    )
    .sort((a, b) => b.month.localeCompare(a.month) || b.id - a.id);
}

function matchIncomes(parsed: Parsed, data: MockData): IncomeDto[] {
  if (parsed.status !== null || parsed.tag !== null) return [];
  const matches = (haystack: string): boolean =>
    parsed.words.every((w) =>
      norm(haystack)
        .split(/\s+/)
        .some((h) => h.startsWith(w))
    );
  return data.incomes
    .filter(
      (i) =>
        matches(i.sourceName) &&
        i.amount >= parsed.min &&
        i.amount <= parsed.max &&
        i.month >= parsed.from &&
        i.month <= parsed.to
    )
    .sort((a, b) => b.month.localeCompare(a.month) || b.id - a.id);
}

export function mockSearch(query: string, requestId: number, data: MockData): SearchResultDto {
  const parsed = parseQuery(query);
  const names = new Map(data.categories.map((c) => [c.id, c.name]));
  const txs = matchTransactions(parsed, data);
  const incomes = matchIncomes(parsed, data);
  return {
    requestId,
    spans: parsed.spans,
    hints: parsed.hints,
    transactions: {
      total: txs.length,
      sum: sum(txs),
      items: txs.slice(0, 8).map((t) => ({
        id: t.id,
        title: t.title,
        categoryId: t.categoryId,
        category: names.get(t.categoryId) ?? '',
        month: t.month,
        date: t.date,
        amount: t.amount,
        status: t.status
      }))
    },
    incomes: {
      total: incomes.length,
      sum: sum(incomes),
      items: incomes.slice(0, 8).map((i) => ({
        id: i.id,
        sourceName: i.sourceName,
        month: i.month,
        date: i.date,
        amount: i.amount,
        status: i.status
      }))
    },
    categories:
      parsed.words.length > 0
        ? data.categories
            .filter((c) =>
              parsed.words.every((w) =>
                norm(c.name)
                  .split(/\s+/)
                  .some((h) => h.startsWith(w))
              )
            )
            .filter((c) => !c.archived)
            .slice(0, 8)
            .map((c) => ({ id: c.id, name: c.name }))
        : [],
    months: []
  };
}

export function mockTransactionList(
  query: string,
  requestId: number,
  data: MockData
): TransactionSearchDto {
  const parsed = parseQuery(query);
  const items = matchTransactions(parsed, data);
  return {
    requestId,
    spans: parsed.spans,
    hints: parsed.hints,
    total: items.length,
    sum: sum(items),
    truncated: false,
    items
  };
}

export function mockIncomeList(query: string, requestId: number, data: MockData): IncomeSearchDto {
  const parsed = parseQuery(query);
  const items = matchIncomes(parsed, data);
  return {
    requestId,
    spans: parsed.spans,
    hints: parsed.hints,
    total: items.length,
    sum: sum(items),
    truncated: false,
    items
  };
}

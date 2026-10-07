import type { TransactionDto } from '$lib/api/bindings';
import { STATUS_ORDER } from '$lib/components/status';

export type SortColumn = 'date' | 'month' | 'title' | 'category' | 'amount' | 'status';

export type TableSort = { column: SortColumn; descending: boolean };

const text = (a: string, b: string): number => a.localeCompare(b, 'ru');

/** Строки таблицы в порядке сортировки (без сортировки — как пришли). Сортирует для показа, суммы не считает. */
export function sortRows(
  rows: readonly TransactionDto[],
  sort: TableSort | null,
  categoryNames: ReadonlyMap<number, string>
): readonly TransactionDto[] {
  if (sort === null) return rows;
  const compare: Record<SortColumn, (a: TransactionDto, b: TransactionDto) => number> = {
    date: (a, b) => (a.date ?? '￿').localeCompare(b.date ?? '￿'),
    month: (a, b) => a.month.localeCompare(b.month),
    title: (a, b) => text(a.title, b.title),
    category: (a, b) =>
      text(categoryNames.get(a.categoryId) ?? '', categoryNames.get(b.categoryId) ?? ''),
    amount: (a, b) => a.amount - b.amount,
    status: (a, b) => STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status)
  };
  const sign = sort.descending ? -1 : 1;
  return [...rows].sort((a, b) => sign * compare[sort.column](a, b));
}

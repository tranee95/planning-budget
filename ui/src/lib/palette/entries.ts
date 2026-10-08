import type {
  IncomeStatusDto,
  SavedFilterDto,
  SearchResultDto,
  TxStatusDto
} from '$lib/api/bindings';
import { formatDay, formatMoney, formatMonth } from '$lib/format';
import { filterCommands, type PaletteCommand } from './filter';

/** Что делает выбранная строка палитры. */
export type PaletteTarget =
  | { type: 'transaction'; id: number; month: string }
  | { type: 'income'; month: string }
  | { type: 'category' }
  | { type: 'month'; month: string }
  | { type: 'table'; screen: 'expenses' | 'incomes'; query: string }
  | { type: 'recent'; query: string }
  | { type: 'command'; command: PaletteCommand };

export interface PaletteEntry {
  /** Уникален в пределах палитры: ключ `{#each}` и часть `id` опции. */
  key: string;
  target: PaletteTarget;
  label: string;
  meta?: string;
  amount?: string;
  /** Статус траты: рисуется `StatusChip`. */
  status?: TxStatusDto;
  /** Подпись статуса дохода. */
  note?: string;
  shortcut?: string;
}

export interface PaletteGroup {
  id: string;
  title: string;
  entries: PaletteEntry[];
}

export const INCOME_STATUS_LABEL: Record<IncomeStatusDto, string> = {
  received: 'Получено',
  expected: 'Ожидается'
};

const plural = (n: number, one: string, few: string, many: string): string => {
  const mod100 = n % 100;
  const mod10 = n % 10;
  if (mod100 >= 11 && mod100 <= 14) return many;
  if (mod10 === 1) return one;
  return mod10 >= 2 && mod10 <= 4 ? few : many;
};

/** «Траты · найдено 9 · 118 881 ₽». */
function groupTitle(name: string, total: number, sum: number): string {
  return `${name} · найдено ${String(total)} · ${formatMoney(sum)}`;
}

const when = (month: string, date: string | null): string =>
  date ? formatDay(date) : formatMonth(month);

/**
 * Группы палитры: результаты поиска, затем команды. Пустой запрос показывает недавние
 * запросы и все команды; пока ответ бэкенда не пришёл, показываются только команды.
 */
export function buildGroups(
  query: string,
  result: SearchResultDto | null,
  commands: readonly PaletteCommand[],
  recent: readonly string[],
  saved: readonly SavedFilterDto[],
  /** Запрос, для которого посчитан `result`: сводка «Показать все» совпадает с его числами. */
  resultQuery: string = query
): PaletteGroup[] {
  const groups: PaletteGroup[] = [];
  const text = query.trim();

  if (text === '' && recent.length > 0) {
    groups.push({
      id: 'recent',
      title: 'Недавние запросы',
      // Ключ становится частью id опции (IDREF): без пробелов и текста запроса.
      entries: recent.map((q, i) => ({
        key: `recent:${String(i)}`,
        target: { type: 'recent', query: q },
        label: q
      }))
    });
  }

  if (text === '' && saved.length > 0) {
    groups.push({
      id: 'saved',
      title: 'Сохранённые фильтры',
      entries: saved.map((f) => ({
        key: `saved:${String(f.id)}`,
        target: { type: 'table', screen: f.screen, query: f.query },
        label: f.name,
        meta: f.query
      }))
    });
  }

  if (result) {
    const { transactions: tx, incomes, categories, months } = result;
    if (tx.items.length > 0) {
      groups.push({
        id: 'transactions',
        title: groupTitle('Траты', tx.total, tx.sum),
        entries: tx.items.map((t) => ({
          key: `tx:${String(t.id)}`,
          target: { type: 'transaction', id: t.id, month: t.month },
          label: t.title,
          meta: `${t.category} · ${when(t.month, t.date)}`,
          amount: formatMoney(t.amount),
          status: t.status
        }))
      });
    }
    if (incomes.items.length > 0) {
      groups.push({
        id: 'incomes',
        title: groupTitle('Доходы', incomes.total, incomes.sum),
        entries: incomes.items.map((i) => ({
          key: `income:${String(i.id)}`,
          target: { type: 'income', month: i.month },
          label: i.sourceName,
          meta: when(i.month, i.date),
          amount: formatMoney(i.amount),
          note: INCOME_STATUS_LABEL[i.status]
        }))
      });
    }
    if (categories.length > 0) {
      groups.push({
        id: 'categories',
        title: 'Категории',
        entries: categories.map((c) => ({
          key: `category:${String(c.id)}`,
          target: { type: 'category' },
          label: c.name
        }))
      });
    }
    if (months.length > 0) {
      groups.push({
        id: 'months',
        title: 'Месяцы',
        entries: months.map((m) => ({
          key: `month:${m.month}`,
          target: { type: 'month', month: m.month },
          label: formatMonth(m.month),
          meta: `${String(m.records)} ${plural(m.records, 'запись', 'записи', 'записей')}`
        }))
      });
    }
  }

  if (result && text !== '') {
    const actions: PaletteEntry[] = [];
    if (result.transactions.total > 0) {
      actions.push({
        key: 'table:expenses',
        target: { type: 'table', screen: 'expenses', query: resultQuery.trim() },
        label: `Показать все траты таблицей (${String(result.transactions.total)})`,
        meta: 'Расходы → фильтр',
        shortcut: 'Ctrl ↵'
      });
    }
    if (result.incomes.total > 0) {
      actions.push({
        key: 'table:incomes',
        target: { type: 'table', screen: 'incomes', query: resultQuery.trim() },
        label: `Показать все доходы таблицей (${String(result.incomes.total)})`,
        meta: 'Доходы → фильтр',
        shortcut: result.transactions.total > 0 ? undefined : 'Ctrl ↵'
      });
    }
    if (actions.length > 0) groups.push({ id: 'actions', title: 'Действия', entries: actions });
  }

  const matched = filterCommands(commands, text);
  if (matched.length > 0) {
    groups.push({
      id: 'commands',
      title: 'Команды',
      entries: matched.map((command) => ({
        key: `command:${command.id}`,
        target: { type: 'command', command },
        label: command.label,
        meta: command.group,
        shortcut: command.shortcut
      }))
    });
  }
  return groups;
}

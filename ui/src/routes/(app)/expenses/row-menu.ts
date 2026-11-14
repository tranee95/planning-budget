import type { TransactionDto, TxStatusDto } from '$lib/api/bindings';
import type { MenuItem } from '$lib/components/menu';
import { STATUS_LABEL, STATUS_ORDER } from '$lib/components/status';

const STATUS_PREFIX = 'status:';

/** Пункты меню строки траты (правый клик). Статусы — выбор из четырёх с отметкой текущего; «Выбрать» только там, где выбор виден (таблица). */
export function rowMenuItems(
  row: TransactionDto,
  checked: boolean,
  selectable: boolean
): MenuItem[] {
  return [
    { id: 'edit', label: 'Изменить' },
    { id: 'tags', label: 'Теги…' },
    ...STATUS_ORDER.map((status, i): MenuItem => ({
      id: STATUS_PREFIX + status,
      label: STATUS_LABEL[status],
      checked: row.status === status,
      separator: i === 0
    })),
    ...(selectable
      ? [{ id: 'check', label: checked ? 'Снять выбор' : 'Выбрать', separator: true }]
      : []),
    { id: 'delete', label: 'Удалить', danger: true, separator: !selectable }
  ];
}

/** Статус из id пункта меню или `null`, если пункт не про статус. */
export function statusOfItem(id: string): TxStatusDto | null {
  if (!id.startsWith(STATUS_PREFIX)) return null;
  return STATUS_ORDER.find((s) => s === id.slice(STATUS_PREFIX.length)) ?? null;
}

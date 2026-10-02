import type { TxStatusDto } from '$lib/api/bindings';

export const STATUS_LABEL: Record<TxStatusDto, string> = {
  paid: 'Оплачено',
  debt: 'Долг',
  unplanned: 'Незапланировано',
  planned: 'План'
};

export const STATUS_ORDER: readonly TxStatusDto[] = ['paid', 'debt', 'unplanned', 'planned'];

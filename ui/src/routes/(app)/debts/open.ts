import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

/** Открывает «Долги» с формой нового долга из траты со статусом «Долг» (сумма и статья берутся из траты). */
export function createDebtFromTransaction(txId: number, month: string): void {
  // Путь разрешён через resolve, запрос дописывается к нему.
  // eslint-disable-next-line svelte/no-navigation-without-resolve
  void goto(`${resolve('/debts')}?fromTx=${String(txId)}&month=${month}`);
}

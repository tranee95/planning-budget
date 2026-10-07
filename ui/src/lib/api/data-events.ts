import { events, type ChangeScope, type DataChanged } from '$lib/api/bindings';

/**
 * Подписка на `data-changed`. Без `scopes` обработчик получает все изменения;
 * с `scopes` — только перечисленные области и `all`. Возвращает синхронный cleanup.
 */
export function onDataChanged(
  handler: (change: DataChanged) => void,
  scopes?: readonly ChangeScope[]
): () => void {
  const off = events.dataChanged.listen(({ payload }) => {
    if (scopes === undefined || payload.scope === 'all' || scopes.includes(payload.scope)) {
      handler(payload);
    }
  });
  return () => {
    void off.then((stop) => {
      stop();
    });
  };
}

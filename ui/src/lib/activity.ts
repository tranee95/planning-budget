import { vaultApi } from '$lib/api/vault';

const THROTTLE_MS = 30_000;

/**
 * Сообщает бэкенду об активности (`activity_ping`) на `pointerdown` и `keydown`,
 * не чаще раза в 30 с: так автоблокировка отсчитывается от реальной работы.
 * Возвращает функцию отписки.
 */
export function startActivityPing(
  target: Window = window,
  now: () => number = Date.now
): () => void {
  let last = Number.NEGATIVE_INFINITY;
  const handler = (): void => {
    const t = now();
    if (t - last < THROTTLE_MS) return;
    last = t;
    void vaultApi.ping().catch(() => {
      // Locked обработает общий слой; пинг не должен шуметь
    });
  };
  target.addEventListener('pointerdown', handler, { passive: true });
  target.addEventListener('keydown', handler, { passive: true });
  return () => {
    target.removeEventListener('pointerdown', handler);
    target.removeEventListener('keydown', handler);
  };
}

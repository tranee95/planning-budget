export interface ListPlacement {
  /** Открыть список над полем. */
  up: boolean;
  /** Максимальная высота списка в тех же единицах, что и входные размеры. */
  maxHeight: number;
}

const MIN_HEIGHT = 96;

/**
 * Куда открыть выпадающий список: вниз, если места хватает, иначе туда, где его больше.
 * Все размеры — в одних единицах (px окна), `viewport` — высота окна.
 */
export function placeList(o: {
  triggerTop: number;
  triggerBottom: number;
  viewport: number;
  desired: number;
  gap?: number;
  margin?: number;
}): ListPlacement {
  const gap = o.gap ?? 4;
  const margin = o.margin ?? 8;
  const below = o.viewport - o.triggerBottom - gap - margin;
  const above = o.triggerTop - gap - margin;
  const up = below < o.desired && above > below;
  const room = up ? above : below;
  return { up, maxHeight: Math.min(o.desired, Math.max(room, MIN_HEIGHT)) };
}

/**
 * Сдвиг по горизонтали, чтобы подсказка шириной `width`, центрированная на `center`,
 * не выходила за окно шириной `viewport`. Положительный сдвиг — вправо.
 */
export function clampShift(o: {
  center: number;
  width: number;
  viewport: number;
  margin?: number;
}): number {
  const margin = o.margin ?? 8;
  const left = o.center - o.width / 2;
  const right = o.center + o.width / 2;
  if (left < margin) return margin - left;
  if (right > o.viewport - margin) return o.viewport - margin - right;
  return 0;
}

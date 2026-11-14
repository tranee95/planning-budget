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
  /** Левая граница области, в которой можно стоять (по умолчанию край окна): например, правее бокового меню. */
  from?: number;
}): number {
  const margin = o.margin ?? 8;
  const from = o.from ?? 0;
  const left = o.center - o.width / 2;
  const right = o.center + o.width / 2;
  if (left < from + margin) return from + margin - left;
  if (right > o.viewport - margin) return o.viewport - margin - right;
  return 0;
}

/** Левый край области контента: подсказки не заходят на боковое меню. 0 вне оболочки приложения. */
export function contentLeft(anchor: Element): number {
  const main = anchor.closest('#main');
  return main === null ? 0 : main.getBoundingClientRect().left;
}

/** Подсказка открывается вверх, если снизу она не помещается, а сверху места больше. */
export function tipFlipsUp(o: {
  triggerTop: number;
  triggerBottom: number;
  height: number;
  viewport: number;
}): boolean {
  const below = o.viewport - o.triggerBottom;
  return below < o.height + 8 && o.triggerTop > below;
}

/**
 * Левый верхний угол контекстного меню шириной `width` и высотой `height`, открытого в точке
 * `x`, `y`: меню прижимается к краям окна, если иначе вышло бы за него.
 */
export function placeMenu(o: {
  x: number;
  y: number;
  width: number;
  height: number;
  viewportWidth: number;
  viewportHeight: number;
  margin?: number;
}): { left: number; top: number } {
  const margin = o.margin ?? 8;
  const left = Math.min(o.x, o.viewportWidth - o.width - margin);
  const top = Math.min(o.y, o.viewportHeight - o.height - margin);
  return { left: Math.max(left, margin), top: Math.max(top, margin) };
}

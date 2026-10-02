import { dur } from '$lib/motion';

/** FLIP для карточек сетки. */

export type FlipParams = {
  /** Смещение, которое карточке задаёт перетаскивание: такие кадры не анимируются. */
  shiftX: number;
  shiftY: number;
  /** Карточку сейчас ведёт указатель. */
  dragging: boolean;
};

/**
 * Запоминает позицию карточки в сетке и при её смене проигрывает переход от старого места
 * к новому. Позиция берётся из `offsetLeft/offsetTop`: ей не мешают прокрутка и `transform`.
 */
export function flip(node: HTMLElement, initial: FlipParams) {
  let lastX = node.offsetLeft + initial.shiftX;
  let lastY = node.offsetTop + initial.shiftY;
  return {
    update(params: FlipParams) {
      const x = node.offsetLeft + params.shiftX;
      const y = node.offsetTop + params.shiftY;
      const duration = dur.base();
      const dx = lastX - x;
      const dy = lastY - y;
      if (!params.dragging && duration > 0 && (dx !== 0 || dy !== 0)) {
        node.animate(
          [{ transform: `translate(${String(dx)}px, ${String(dy)}px)` }, { transform: 'none' }],
          { duration, easing: 'cubic-bezier(.22, 1, .36, 1)' }
        );
      }
      lastX = x;
      lastY = y;
    }
  };
}

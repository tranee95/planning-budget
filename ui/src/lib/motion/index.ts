import { expoOut, quartOut } from 'svelte/easing';
import { fade, fly, type TransitionConfig } from 'svelte/transition';

/** Длительность из токена в мс. Токены обнуляются при reduced-motion, поэтому читаем при каждом вызове. */
export function token(name: string): number {
  if (typeof document === 'undefined') return 0;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name);
  return Number.parseFloat(value) || 0;
}

export const dur = {
  fast: () => token('--dur-fast'),
  base: () => token('--dur-base'),
  pop: () => token('--dur-pop'),
  rise: () => token('--dur-rise'),
  slow: () => token('--dur-slow'),
  drawer: () => token('--dur-drawer')
};

type Direction = { direction?: 'in' | 'out' | 'both' };

/** Уход короче появления: интерфейс не заставляет ждать, пока исчезнет то, что уже не нужно. */
const OUT = 0.55;

/** Задержка i-го элемента списка; хвост длиннее 12 элементов не задерживается. */
export function stagger(index: number): number {
  return Math.min(index, 12) * token('--stagger');
}

/** Появление карточки: opacity 0→1, подъём на 12 px. */
export function rise(node: Element, { delay = 0 }: { delay?: number } = {}): TransitionConfig {
  return fly(node, { y: 12, duration: dur.rise(), delay, easing: quartOut });
}

/**
 * Смена раздела: старый экран быстро гаснет, новый появляется следом с подъёмом на 10 px.
 * Оба лежат в одной ячейке сетки (`.content`), поэтому раскладка не прыгает.
 */
export function routeIn(node: Element): TransitionConfig {
  return fly(node, { y: 10, duration: dur.base(), delay: dur.fast() * OUT, easing: quartOut });
}

export function routeOut(node: Element): TransitionConfig {
  return fade(node, { duration: dur.fast() * OUT, easing: quartOut });
}

/** Смена месяца: сдвиг на 12 px в сторону направления, предыдущий месяц гаснет без сдвига. */
export function monthSlide(node: Element, { direction }: { direction: 1 | -1 }): TransitionConfig {
  return fly(node, {
    x: 12 * direction,
    duration: dur.base(),
    delay: dur.fast() * OUT,
    easing: quartOut
  });
}

export function monthOut(node: Element): TransitionConfig {
  return fade(node, { duration: dur.fast() * OUT, easing: quartOut });
}

/**
 * Название месяца в шапке: вертикальный «ролл». Новое название въезжает снизу при шаге вперёд и сверху
 * при шаге назад, прежнее уезжает в обратную сторону.
 */
export function monthRoll(
  node: Element,
  { direction }: { direction: 1 | -1 },
  { direction: phase }: Direction = {}
): TransitionConfig {
  const leaving = phase === 'out';
  return fly(node, {
    y: (leaving ? -10 : 10) * direction,
    duration: dur.base(),
    easing: quartOut
  });
}

export function scrimFade(
  node: Element,
  _params?: object,
  { direction }: Direction = {}
): TransitionConfig {
  return fade(node, {
    duration: direction === 'out' ? dur.fast() : dur.base(),
    easing: quartOut
  });
}

/**
 * Модалка и палитра: подъём на 14 px, масштаб .97 → 1 и появление; уход быстрее и короче.
 * Сохраняет собственный `transform` узла (центрирование).
 */
export function pop(
  node: Element,
  _params?: object,
  { direction }: Direction = {}
): TransitionConfig {
  const leaving = direction === 'out';
  const base = getComputedStyle(node).transform;
  const own = base === 'none' ? '' : base;
  return {
    // появление 200 мс, закрытие быстрее — 120 мс
    duration: leaving ? dur.fast() : dur.base(),
    easing: leaving ? quartOut : expoOut,
    css: (t, u) =>
      `opacity: ${String(t)}; transform: ${own} translateY(${String(u * 8)}px) scale(${String(0.96 + 0.04 * t)});`
  };
}

/** Правая панель: выезжает справа целиком (translateX 100% → 0) за 280 мс, закрывается за 200 мс. */
export function sheetSlide(
  node: Element,
  _params?: object,
  { direction }: Direction = {}
): TransitionConfig {
  const leaving = direction === 'out';
  return {
    duration: leaving ? dur.base() : dur.drawer(),
    easing: quartOut,
    css: (t, u) => `transform: translateX(${String(u * 100)}%);`
  };
}

/** Тост: въезжает снизу на 12 px, уходит коротким fade. */
export function toastSlide(
  node: Element,
  _params?: object,
  { direction }: Direction = {}
): TransitionConfig {
  if (direction === 'out') return fade(node, { duration: dur.base() * OUT, easing: quartOut });
  return fly(node, { y: 12, duration: dur.base(), easing: expoOut });
}

/**
 * Новая строка списка (трата, доход): раскрывается по высоте и один раз подсвечивается фоном,
 * подсветка гаснет за 1,2 с. Соседи сдвигаются сами, пока высота растёт.
 * Высота здесь — осознанное исключение из «только transform и opacity»: у строки нет обёртки для
 * grid-template-rows, а строк мало.
 */
export function rowIn(node: HTMLElement): TransitionConfig {
  const height = node.offsetHeight;
  if (token('--dur-base') > 0) {
    node.animate([{ backgroundColor: 'var(--accent-soft)' }, { backgroundColor: 'transparent' }], {
      duration: 1200,
      easing: 'ease-out'
    });
  }
  return {
    duration: dur.base(),
    easing: quartOut,
    css: (t) => `overflow: hidden; height: ${String(t * height)}px; opacity: ${String(t)};`
  };
}

/** Удаляемая строка схлопывается, соседи подъезжают на её место. */
export function rowOut(node: HTMLElement): TransitionConfig {
  const height = node.offsetHeight;
  return {
    duration: dur.base(),
    easing: quartOut,
    css: (t) => `overflow: hidden; height: ${String(t * height)}px; opacity: ${String(t)};`
  };
}

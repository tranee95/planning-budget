const FOCUSABLE =
  'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

let locks = 0;
let restore: (() => void) | null = null;

/**
 * Блокирует прокрутку страницы под оверлеем без сдвига контента: место полосы прокрутки (она же
 * резервируется `scrollbar-gutter: stable`) переходит в padding, а слой оверлея получает всю ширину окна
 * и панель встаёт вплотную к краю. Вложенные оверлеи считаются счётчиком.
 */
export function lockScroll(): () => void {
  if (locks++ === 0) {
    const root = document.documentElement;
    const width = window.innerWidth - root.clientWidth;
    const before = {
      overflow: root.style.overflow,
      gutter: root.style.scrollbarGutter,
      padding: root.style.paddingInlineEnd
    };
    root.style.overflow = 'hidden';
    root.style.scrollbarGutter = 'auto';
    root.style.paddingInlineEnd = `${String(Math.max(width, 0))}px`;
    restore = () => {
      root.style.overflow = before.overflow;
      root.style.scrollbarGutter = before.gutter;
      root.style.paddingInlineEnd = before.padding;
    };
  }
  let released = false;
  return () => {
    if (released) return;
    released = true;
    if (--locks === 0) {
      restore?.();
      restore = null;
    }
  };
}

/**
 * Для диалогов: удерживает Tab внутри узла, закрывает по Esc и возвращает фокус туда,
 * где он был до открытия.
 */
export function trapFocus(node: HTMLElement, onclose: () => void) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  let close = onclose;
  const unlock = lockScroll();

  const items = (): HTMLElement[] => [...node.querySelectorAll<HTMLElement>(FOCUSABLE)];

  if (!node.contains(document.activeElement)) (items()[0] ?? node).focus();

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      // Открытый список combobox закрывается сам, диалог остаётся.
      if (event.target instanceof HTMLElement && event.target.dataset.escapeClosesList === 'true') {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (event.key !== 'Tab') return;
    const list = items();
    const first = list[0];
    const last = list[list.length - 1];
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', onKeydown);
  return {
    update(next: () => void) {
      close = next;
    },
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      unlock();
      previous?.focus();
    }
  };
}

const FOCUSABLE =
  'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

/**
 * Для диалогов: удерживает Tab внутри узла, закрывает по Esc и возвращает фокус туда,
 * где он был до открытия.
 */
export function trapFocus(node: HTMLElement, onclose: () => void) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  let close = onclose;

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
      previous?.focus();
    }
  };
}

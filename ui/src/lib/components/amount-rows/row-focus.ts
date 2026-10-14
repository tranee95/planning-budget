/**
 * Фокус ушёл из строки правки или добавления в другую область окна. Переход между полями
 * строки и переключение окна (Alt+Tab) уходом не считаются: ввод в них не должен сохраняться.
 */
export function leftRow(event: FocusEvent, row: HTMLElement | undefined): boolean {
  const next = event.relatedTarget;
  if (next instanceof Node && row?.contains(next)) return false;
  return document.hasFocus();
}

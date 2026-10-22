/**
 * Delete на сфокусированной строке удаляет её (с возвратом тостом и Ctrl+Z, как «×»).
 * Поля ввода не затрагиваются: там Delete стирает символ.
 */
export function deleteOnKey(event: KeyboardEvent, remove: () => void): void {
  if (event.key !== 'Delete' || event.defaultPrevented) return;
  const target = event.target;
  if (!(target instanceof HTMLElement)) return;
  if (target.closest('input, textarea, select, [contenteditable="true"]') !== null) return;
  event.preventDefault();
  remove();
}

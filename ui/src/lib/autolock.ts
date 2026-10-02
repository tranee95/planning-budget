/** Пороги автоблокировки; `0` — никогда. */
export const AUTOLOCK_OPTIONS: readonly { value: string; label: string }[] = [
  { value: '1', label: '1 минута' },
  { value: '5', label: '5 минут' },
  { value: '15', label: '15 минут' },
  { value: '30', label: '30 минут' },
  { value: '0', label: 'Никогда' }
];

function minutesWord(n: number): string {
  const last = n % 10;
  if (n % 100 >= 11 && n % 100 <= 14) return 'минут';
  if (last === 1) return 'минуту';
  if (last >= 2 && last <= 4) return 'минуты';
  return 'минут';
}

/** Строка для экрана входа: по настройке, которую бэкенд дублирует в `ui-prefs.json`. */
export function autolockLine(minutes: number): string {
  return minutes === 0
    ? 'Автоблокировка отключена'
    : `Автоблокировка через ${String(minutes)} ${minutesWord(minutes)} бездействия`;
}

/** Пункты списка; значение вне таблицы (старая настройка) добавляется, чтобы список не оставался пустым. */
export function autolockOptions(current: string): readonly { value: string; label: string }[] {
  if (AUTOLOCK_OPTIONS.some((o) => o.value === current)) return AUTOLOCK_OPTIONS;
  return [{ value: current, label: `${current} мин` }, ...AUTOLOCK_OPTIONS];
}

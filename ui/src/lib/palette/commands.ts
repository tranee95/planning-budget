import type { Theme } from '$lib/api/bindings';
import type { PaletteCommand } from './filter';

const THEMES: readonly { value: Theme; label: string; keywords: string }[] = [
  { value: 'light', label: 'светлая', keywords: 'светлый день light' },
  { value: 'dark', label: 'тёмная', keywords: 'тёмный ночная ночной dark' },
  { value: 'system', label: 'как в системе', keywords: 'системная авто auto system' }
];

/** Команды смены темы: «Тема: тёмная» и т. д., с синонимами для поиска. */
export function themeCommands(apply: (theme: Theme) => void): PaletteCommand[] {
  return THEMES.map((t) => ({
    id: `theme-${t.value}`,
    label: `Тема: ${t.label}`,
    group: 'Вид',
    keywords: `тема оформление theme ${t.keywords}`,
    run: () => {
      apply(t.value);
    }
  }));
}

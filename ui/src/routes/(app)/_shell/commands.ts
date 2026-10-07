import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { helpItem, sections, settingsItem } from '$lib/nav';
import { themeCommands } from '$lib/palette/commands';
import type { PaletteTarget } from '$lib/palette/entries';
import type { PaletteCommand } from '$lib/palette/filter';
import { shortcutLabel } from '$lib/shortcuts';
import { month } from '$lib/stores/month.svelte';
import { onboarding } from '$lib/stores/onboarding.svelte';
import { prefs } from '$lib/stores/prefs.svelte';
import { session } from '$lib/stores/session.svelte';
import { toasts } from '$lib/stores/toasts.svelte';

/** Команда работает только при разблокированном сейфе; ошибка превращается в тост. */
export const guarded = (action: () => void | Promise<void>) => (): void => {
  if (session.phase !== 'unlocked') return;
  Promise.resolve(action()).catch(() => {
    toasts.push({ message: 'Не удалось выполнить команду', kind: 'error' });
  });
};

/** Переход к результату поиска: месяц записи становится выбранным. */
export function openTarget(target: PaletteTarget): void {
  if (target.type === 'transaction') {
    month.set(target.month);
    // Путь разрешён через resolve, запрос дописывается к нему.
    // eslint-disable-next-line svelte/no-navigation-without-resolve
    void goto(`${resolve('/expenses')}?edit=${String(target.id)}`);
  } else if (target.type === 'income') {
    month.set(target.month);
    void goto(resolve('/incomes'));
  } else if (target.type === 'month') {
    month.set(target.month);
    void goto(resolve('/'));
  } else if (target.type === 'table') {
    const path = resolve(target.screen === 'expenses' ? '/expenses' : '/incomes');
    // eslint-disable-next-line svelte/no-navigation-without-resolve
    void goto(`${path}?q=${encodeURIComponent(target.query)}`);
  } else if (target.type === 'category') {
    void goto(resolve('/categories'));
  }
}

/** Команды палитры ⌘K: переходы по реестру разделов, тема, блокировка. */
export function buildCommands(): PaletteCommand[] {
  return [
    ...sections.map((s, i) => ({
      id: `go-${s.href}`,
      label: s.label,
      group: 'Переход',
      shortcut: String(i + 1),
      run: () => void goto(resolve(s.href))
    })),
    {
      id: `go-${settingsItem.href}`,
      label: settingsItem.label,
      group: 'Переход',
      run: () => void goto(resolve(settingsItem.href))
    },
    {
      id: 'help',
      label: 'Как пользоваться',
      group: 'Справка',
      shortcut: '?',
      run: () => void goto(resolve(helpItem.href))
    },
    {
      id: 'plan-wizard',
      label: 'Спланировать месяц (мастер)',
      group: 'Справка',
      run: guarded(() => {
        onboarding.openWizard();
      })
    },
    ...themeCommands((theme) => {
      guarded(() => prefs.set({ theme }))();
    }),
    {
      id: 'lock',
      label: 'Заблокировать',
      group: 'Сейф',
      shortcut: shortcutLabel({ code: 'KeyL', mod: true }),
      run: guarded(() => session.lock())
    }
  ];
}

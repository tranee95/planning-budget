import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { helpItem, sections } from '$lib/nav';
import { shortcuts } from '$lib/shortcuts';
import { month } from '$lib/stores/month.svelte';
import { session } from '$lib/stores/session.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { guarded } from './commands';

/** Горячие клавиши оболочки: палитра, блокировка, месяц, история, цифры разделов. Возвращает cleanup. */
export function registerShellShortcuts(togglePalette: () => void): () => void {
  const offs = [
    shortcuts.register({
      id: 'palette',
      code: 'KeyK',
      mod: true,
      inInput: true,
      label: 'Поиск и команды',
      run: guarded(togglePalette)
    }),
    shortcuts.register({
      id: 'lock',
      code: 'KeyL',
      mod: true,
      inInput: true,
      label: 'Заблокировать',
      run: guarded(() => session.lock())
    }),
    shortcuts.register({
      id: 'month-prev',
      code: 'ArrowLeft',
      label: 'Предыдущий месяц',
      run: () => {
        month.shift(-1);
      }
    }),
    shortcuts.register({
      id: 'month-next',
      code: 'ArrowRight',
      label: 'Следующий месяц',
      run: () => {
        month.shift(1);
      }
    }),
    shortcuts.register({
      id: 'month-today',
      code: 'KeyT',
      label: 'Текущий месяц',
      run: () => {
        month.today();
      }
    }),
    shortcuts.register({
      id: 'history.undo',
      code: 'KeyZ',
      mod: true,
      label: 'Отменить',
      run: () => {
        void undoStack.undo();
      }
    }),
    shortcuts.register({
      id: 'history.redo',
      code: 'KeyZ',
      mod: true,
      shift: true,
      label: 'Повторить',
      run: () => {
        void undoStack.redo();
      }
    }),
    // «?»: Shift+/ на латинской раскладке, Shift+7 на русской (сопоставление по коду клавиши).
    ...['Slash', 'Digit7'].map((code) =>
      shortcuts.register({
        id: `help-${code}`,
        code,
        shift: true,
        label: 'Справка',
        run: () => void goto(resolve(helpItem.href))
      })
    ),
    ...sections.map((s, i) =>
      shortcuts.register({
        id: `section-${s.href}`,
        code: `Digit${String(i + 1)}`,
        label: s.label,
        run: () => void goto(resolve(s.href))
      })
    )
  ];
  return () => {
    for (const off of offs) off();
  };
}

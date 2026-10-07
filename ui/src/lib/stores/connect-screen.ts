import { onDataChanged } from '$lib/api/data-events';
import { shortcuts, type Shortcut } from '$lib/shortcuts';

/** Пустой список месяцев в `data-changed` — изменились данные всех месяцев. */
export const affectsMonth = (months: readonly string[], current: string): boolean =>
  current !== '' && (months.length === 0 || months.includes(current));

interface ScreenConnection {
  /** Горячая клавиша экрана («Новая трата», «Новый доход»). */
  shortcut: Pick<Shortcut, 'id' | 'code' | 'label' | 'run'>;
  /** Месяц, который показан сейчас. */
  month: () => string;
  /** Список не привязан к месяцу (результат поиска): перезагружать при любом изменении. */
  anyMonth?: () => boolean;
  /** Перечитать экран: вызывается, когда `data-changed` затронул показанный месяц. */
  reload: () => void;
}

/** Подписки экрана данных: горячая клавиша и перезагрузка по `data-changed`. Возвращает cleanup. */
export function connectScreen({ shortcut, month, anyMonth, reload }: ScreenConnection): () => void {
  const offShortcut = shortcuts.register(shortcut);
  const offData = onDataChanged(({ months }) => {
    if (anyMonth?.() === true || affectsMonth(months, month())) reload();
  });
  return () => {
    offShortcut();
    offData();
  };
}

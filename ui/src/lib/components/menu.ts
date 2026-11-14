export type MenuItem = {
  id: string;
  label: string;
  /** Подпись справа: горячая клавиша или текущее значение. */
  hint?: string;
  /** Пункт отмечен (выбор из нескольких, например текущий статус). */
  checked?: boolean;
  danger?: boolean;
  disabled?: boolean;
  /** Линия-разделитель перед пунктом. */
  separator?: boolean;
};

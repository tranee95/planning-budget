import { shortcutLabel } from '$lib/shortcuts';

/** Подсказки при первом запуске: три шага, без выдуманных возможностей. */
export type Tip = { title: string; text: string; keys?: readonly string[] };

const PALETTE_KEYS = shortcutLabel({ code: 'KeyK', mod: true });

export const TIPS: readonly Tip[] = [
  {
    title: 'Добавляйте траты за секунды',
    text: 'На экране «Расходы» нажмите N: сумма, название, категория, Enter.',
    keys: ['N']
  },
  {
    title: 'Задайте лимиты',
    text: 'В «Категориях» укажите месячный лимит по каждой статье, и «Обзор» покажет остаток.'
  },
  {
    title: 'Ищите и управляйте с клавиатуры',
    text: `${PALETTE_KEYS} ищет по тратам, доходам и категориям и выполняет команды.`,
    keys: PALETTE_KEYS.split(' ')
  }
];

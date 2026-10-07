import type { Component } from 'svelte';
import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
import ReceiptText from '@lucide/svelte/icons/receipt-text';
import Wallet from '@lucide/svelte/icons/wallet';
import Tag from '@lucide/svelte/icons/tag';
import ChartColumn from '@lucide/svelte/icons/chart-column';
import PiggyBank from '@lucide/svelte/icons/piggy-bank';
import HandCoins from '@lucide/svelte/icons/hand-coins';
import Settings from '@lucide/svelte/icons/settings';
import CircleHelp from '@lucide/svelte/icons/circle-help';

export interface NavItem {
  href:
    | '/'
    | '/expenses'
    | '/incomes'
    | '/categories'
    | '/analytics'
    | '/savings'
    | '/debts'
    | '/help'
    | '/settings';
  label: string;
  icon: Component<{ size?: number; strokeWidth?: number; 'aria-hidden'?: 'true' }>;
  /** Заголовок топбара, если он отличается от названия пункта меню. */
  title?: string;
  /** Экран показывает данные выбранного месяца: переключатель месяца виден, смена месяца перерисовывает экран. */
  monthScoped: boolean;
}

/** Разделы с экранами. Горячие клавиши 1…N и команды ⌘K совпадают с порядком; новый раздел — одна запись здесь. */
export const sections: readonly NavItem[] = [
  { href: '/', label: 'Обзор', icon: LayoutDashboard, monthScoped: true },
  { href: '/expenses', label: 'Расходы', icon: ReceiptText, monthScoped: true },
  { href: '/incomes', label: 'Доходы', icon: Wallet, monthScoped: true },
  {
    href: '/categories',
    label: 'Категории',
    icon: Tag,
    title: 'Категории и лимиты',
    monthScoped: true
  },
  { href: '/analytics', label: 'Аналитика', icon: ChartColumn, monthScoped: false },
  { href: '/savings', label: 'Сбережения', icon: PiggyBank, monthScoped: false },
  { href: '/debts', label: 'Долги', icon: HandCoins, monthScoped: true }
];

/** «Настройки» стоят в сайдбаре отдельно и горячей цифры не имеют. */
export const settingsItem: NavItem = {
  href: '/settings',
  label: 'Настройки',
  icon: Settings,
  monthScoped: false
};

/** «Справка» стоит в сайдбаре отдельно: клавиша `?`, команда ⌘K «Как пользоваться». */
export const helpItem: NavItem = {
  href: '/help',
  label: 'Справка',
  icon: CircleHelp,
  monthScoped: false
};

/** Запись реестра для пути (для экранов вне реестра — `undefined`). */
export function routeMeta(pathname: string): NavItem | undefined {
  return [...sections, helpItem, settingsItem].find((s) => s.href === pathname);
}

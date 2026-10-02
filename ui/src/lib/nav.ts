import type { Component } from 'svelte';
import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
import ReceiptText from '@lucide/svelte/icons/receipt-text';
import Wallet from '@lucide/svelte/icons/wallet';
import Tag from '@lucide/svelte/icons/tag';
import ChartColumn from '@lucide/svelte/icons/chart-column';
import Landmark from '@lucide/svelte/icons/landmark';

export interface NavItem {
  href: '/' | '/expenses' | '/incomes' | '/categories' | '/analytics' | '/bonds' | '/settings';
  label: string;
  icon: Component<{ size?: number; strokeWidth?: number; 'aria-hidden'?: 'true' }>;
}

/** Разделы с экранами. Горячие клавиши 1…N совпадают с порядком; новые экраны добавляются в конец перед «Настройками». */
export const sections: readonly NavItem[] = [
  { href: '/', label: 'Обзор', icon: LayoutDashboard },
  { href: '/expenses', label: 'Расходы', icon: ReceiptText },
  { href: '/incomes', label: 'Доходы', icon: Wallet },
  { href: '/categories', label: 'Категории', icon: Tag },
  { href: '/analytics', label: 'Аналитика', icon: ChartColumn },
  { href: '/bonds', label: 'Облигации', icon: Landmark }
];

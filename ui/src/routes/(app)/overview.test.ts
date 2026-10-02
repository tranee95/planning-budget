import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockBudget } from '$lib/api/mock/budget';
import { installMocks } from '$lib/api/mock/install';
import { categories } from '$lib/stores/categories.svelte';
import { OverviewVm } from './overview.svelte';

beforeEach(() => {
  installMocks();
  resetMockBudget();
});

afterEach(() => {
  clearMocks();
  categories.reset();
});

it('load: сводка, год, настройки и лимиты', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.error).toBeNull();
  expect(vm.summary?.month).toBe('2026-09');
  expect(vm.settings?.savingsMinBp).toBe(1300);
  expect(vm.series).toHaveLength(12);
});

it('лимиты отсортированы по использованию, сбережения не входят', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  const usage = vm.limitLines.map((l) => l.row.usage ?? 0);
  expect(usage).toEqual([...usage].sort((a, b) => b - a));
  expect(vm.limitLines.every((l) => l.category.kind !== 'savings')).toBe(true);
});

it('подзаголовок: в мок-данных превышений нет', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  expect(vm.subtitle).toBe('Сентябрь: все категории в лимитах');
});

it('пустой месяц: нет дохода, сравнивать расходы не с чем', async () => {
  const vm = new OverviewVm();
  await vm.load('2025-01');
  expect(vm.savingsHint).toBe('нет дохода в месяце');
  expect(vm.expensesDelta).toBeNull();
});

it('«12 мес» берёт ровно двенадцать месяцев до выбранного', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.setRange('12m');
  expect(vm.series).toHaveLength(12);
  expect(vm.series.at(-1)?.month).toBe('2026-09');
});

it('«Всё» оставляет только месяцы с данными', async () => {
  const vm = new OverviewVm();
  await vm.load('2026-09');
  await vm.setRange('all');
  // 2024 — история «Ленты» из мока (E2E 4); пустые 2025 и месяцы 2026 до сентября отброшены.
  expect(vm.series.map((m) => m.month)).toEqual([
    ...Array.from({ length: 8 }, (_, i) => `2024-0${String(i + 1)}`),
    '2026-09'
  ]);
});

it('смена периода графика во время загрузки не обрывает загрузку', async () => {
  const vm = new OverviewVm();
  const loading = vm.load('2026-09');
  const ranging = vm.setRange('12m');
  await Promise.all([loading, ranging]);
  expect(vm.loading).toBe(false);
  expect(vm.summary?.month).toBe('2026-09');
});

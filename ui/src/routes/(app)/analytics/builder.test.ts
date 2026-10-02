import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockDashboards } from '$lib/api/mock/dashboards';
import { installMocks } from '$lib/api/mock/install';
import { toasts } from '$lib/stores/toasts.svelte';
import { AnalyticsVm } from './analytics.svelte';
import { BuilderVm, defaultSize } from './builder.svelte';

beforeEach(() => {
  installMocks();
  resetMockDashboards();
  toasts.clear();
});

afterEach(() => {
  clearMocks();
});

const settled = async (vm: BuilderVm): Promise<void> => {
  await expect.poll(() => vm.preview.status).not.toBe('loading');
};

it('создание: превью строится, значения, делающие график недопустимым, отключены', async () => {
  const vm = new BuilderVm();
  vm.beginCreate();
  await settled(vm);
  expect(vm.preview.status).toBe('ready');
  expect(vm.canSave).toBe(true);
  expect(vm.blocked('type:donut')).toBe('errors.chart.donut');
  expect(vm.blocked('type:line')).toBeNull();
  expect(vm.blocked('metric:limit_usage')).toBe('errors.chart.limit_usage_by_category');
});

it('смена разбивки открывает кольцо и меняет превью', async () => {
  const vm = new BuilderVm();
  vm.beginCreate();
  await settled(vm);
  vm.change({ groupBy: 'kind' });
  await expect.poll(() => vm.blocked('type:donut')).toBeNull();
  vm.change({ type: 'donut' });
  await settled(vm);
  expect(vm.preview.status).toBe('ready');
});

it('недопустимое описание: причина в превью, сохранение закрыто', async () => {
  const vm = new BuilderVm();
  vm.beginCreate();
  vm.change({ type: 'stacked_bar' });
  await expect.poll(() => vm.problem).toBe('errors.chart.stacked_without_series');
  expect(vm.canSave).toBe(false);
  expect(vm.preview.status).toBe('error');
  // значение, упирающееся в ту же причину, не помечается виноватым
  expect(vm.blocked('type:stacked_bar')).toBeNull();
});

it('серии по показателям: нужно два показателя, переключатель их набирает', () => {
  const vm = new BuilderVm();
  vm.beginCreate();
  vm.setSeries('metric');
  expect(vm.draft.metrics).toEqual(['income', 'expenses']);
  vm.toggleMetric('savings');
  expect(vm.draft.metrics).toEqual(['income', 'expenses', 'savings']);
  vm.toggleMetric('income');
  expect(vm.draft.metrics).toEqual(['expenses', 'savings']);
  expect(vm.draft.metric).toBe('expenses');
});

it('правка существующей карточки сохраняется и пересчитывает данные', async () => {
  const analytics = new AnalyticsVm();
  await analytics.load();
  const card = analytics.cards[0];
  if (card === undefined) throw new Error('нет карточек');
  const vm = new BuilderVm();
  vm.beginEdit(card.id, card.spec);
  expect(vm.mode).toEqual({ kind: 'edit', cardId: card.id });
  vm.change({ title: 'Переименован' });
  expect(await analytics.updateChart(card.id, vm.draft)).toBe(true);
  expect(analytics.cards.find((c) => c.id === card.id)?.spec.title).toBe('Переименован');
  expect(analytics.data.get(card.id)?.status).toBe('ready');
});

it('новый график добавляется под остальными', async () => {
  const analytics = new AnalyticsVm();
  await analytics.load();
  const vm = new BuilderVm();
  vm.beginCreate();
  const { w, h } = defaultSize(vm.draft.type);
  expect(await analytics.addChart(vm.draft, w, h)).toBe(true);
  expect(analytics.cards).toHaveLength(10);
  expect(analytics.cards.at(-1)?.spec.title).toBe('Новый график');
});

it('close отменяет отложенное превью', async () => {
  const vm = new BuilderVm();
  vm.beginCreate();
  vm.close();
  await new Promise((r) => setTimeout(r, 250));
  expect(vm.open).toBe(false);
  expect(vm.preview.status).toBe('loading');
});

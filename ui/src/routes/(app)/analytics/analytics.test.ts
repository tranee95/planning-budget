import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockDashboards } from '$lib/api/mock/dashboards';
import { installMocks } from '$lib/api/mock/install';
import { toasts } from '$lib/stores/toasts.svelte';
import { undoStack } from '$lib/stores/undo.svelte';
import { AnalyticsVm } from './analytics.svelte';

beforeEach(() => {
  installMocks();
  resetMockDashboards();
  toasts.clear();
  undoStack.clear();
});

afterEach(() => {
  clearMocks();
});

async function loaded(): Promise<AnalyticsVm> {
  const vm = new AnalyticsVm();
  await vm.load();
  return vm;
}

it('load: основной дашборд, девять карточек и данные для каждой', async () => {
  const vm = await loaded();
  expect(vm.active?.name).toBe('Мой бюджет');
  expect(vm.cards).toHaveLength(9);
  expect(vm.layout).toHaveLength(9);
  expect([...vm.data.values()].every((d) => d.status === 'ready')).toBe(true);
  expect(vm.loading).toBe(false);
});

it('commit: перенос карточки сохраняется и прижимает остальные', async () => {
  const vm = await loaded();
  const [first, second] = vm.cards;
  if (first === undefined || second === undefined) throw new Error('нет карточек');
  await vm.commit(second.id, { x: 0, y: 0 });
  expect(vm.layout.find((b) => b.id === second.id)).toMatchObject({ x: 0, y: 0 });
  expect(vm.layout.find((b) => b.id === first.id)?.y).toBeGreaterThan(0);

  const reloaded = new AnalyticsVm();
  await reloaded.load();
  expect(reloaded.cards.find((c) => c.id === second.id)).toMatchObject({ x: 0, y: 0 });
});

it('preview не пишет в базу, cancelPreview возвращает сохранённое', async () => {
  const vm = await loaded();
  const first = vm.cards[0];
  if (first === undefined) throw new Error('нет карточек');
  vm.preview(first.id, { x: 6, y: 20 });
  expect(vm.layout.find((b) => b.id === first.id)?.y).toBe(20);
  vm.cancelPreview();
  expect(vm.layout.find((b) => b.id === first.id)).toMatchObject({ x: 0, y: 0 });
  expect(vm.cards[0]).toMatchObject({ x: 0, y: 0 });
});

it('resize: пресет L занимает всю ширину', async () => {
  const vm = await loaded();
  const second = vm.cards[1];
  if (second === undefined) throw new Error('нет карточек');
  await vm.resize(second.id, 'L');
  expect(vm.layout.find((b) => b.id === second.id)).toMatchObject({ x: 0, w: 12 });
});

it('remove и отмена возвращают карточку на прежнее место', async () => {
  const vm = await loaded();
  const target = vm.cards[1];
  if (target === undefined) throw new Error('нет карточек');
  await vm.remove(target.id);
  expect(vm.cards).toHaveLength(8);
  expect(toasts.items.at(-1)?.action?.label).toBe('Отменить');

  await undoStack.undo();
  expect(vm.cards).toHaveLength(9);
  const restored = vm.cards.find((c) => c.spec.title === target.spec.title);
  expect(restored).toMatchObject({ x: target.x, y: target.y, w: target.w, h: target.h });
});

it('duplicate добавляет копию под остальными', async () => {
  const vm = await loaded();
  const first = vm.cards[0];
  if (first === undefined) throw new Error('нет карточек');
  await vm.duplicate(first.id);
  expect(vm.cards).toHaveLength(10);
  expect(vm.cards.some((c) => c.spec.title === `${first.spec.title} (копия)`)).toBe(true);
});

it('несколько дашбордов: создание выбирает новый, удаление возвращает основной', async () => {
  const vm = await loaded();
  expect(await vm.createDashboard('Отпуск')).toBe(true);
  expect(vm.active?.name).toBe('Отпуск');
  expect(vm.cards).toHaveLength(0);

  expect(await vm.renameDashboard(vm.activeId ?? 0, 'Поездки')).toBe(true);
  expect(vm.active?.name).toBe('Поездки');

  await vm.deleteDashboard(vm.activeId ?? 0);
  expect(vm.dashboards).toHaveLength(1);
  expect(vm.active?.name).toBe('Мой бюджет');
  expect(vm.cards).toHaveLength(9);
});

it('последний дашборд удалить нельзя: тост с причиной', async () => {
  const vm = await loaded();
  await vm.deleteDashboard(vm.activeId ?? 0);
  expect(vm.dashboards).toHaveLength(1);
  expect(toasts.items.at(-1)?.message).toBe('Последний дашборд удалить нельзя.');
});

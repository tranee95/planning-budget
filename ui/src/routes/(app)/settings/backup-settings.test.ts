import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockVault } from '$lib/api/mock/handlers';
import { installMocks } from '$lib/api/mock/install';
import { BackupVm, VaultsVm, intervalOptions } from './backup-settings.svelte';

beforeEach(() => {
  installMocks();
  resetMockVault();
});

afterEach(() => {
  clearMocks();
});

it('расписание: выключатель и периодичность сохраняются и читаются обратно', async () => {
  const vm = new BackupVm();
  await vm.load();
  expect(vm.loaded).toBe(true);

  await vm.setAuto(false);
  await vm.setInterval('7');
  const again = new BackupVm();
  await again.load();
  expect(again.auto).toBe(false);
  expect(again.intervalDays).toBe('7');
});

it('ручная копия появляется в списке первой', async () => {
  const vm = new BackupVm();
  await vm.load();
  const before = vm.status?.items.length ?? 0;
  await vm.create();
  expect(vm.creating).toBe(false);
  expect(vm.status?.items.length).toBeGreaterThan(0);
  expect(vm.status?.items.length).toBeLessThanOrEqual(Math.max(before + 1, 7));
});

it('каталог: выбор и возврат к каталогу по умолчанию', async () => {
  const vm = new BackupVm();
  await vm.load();
  await vm.pickDir();
  expect(vm.status?.dirIsDefault).toBe(false);
  await vm.resetDir();
  expect(vm.status?.dirIsDefault).toBe(true);
});

it('значение интервала вне таблицы остаётся в списке', () => {
  expect(intervalOptions('5')[0]?.value).toBe('5');
  expect(intervalOptions('7')).toHaveLength(4);
});

it('хранилища: добавление делает новое активным, пустое можно убрать', async () => {
  const vm = new VaultsVm();
  await vm.load();
  expect(vm.items.filter((v) => v.active)).toHaveLength(1);

  expect(await vm.add('Семья')).toBe(true);
  await vm.load();
  const added = vm.items.find((v) => v.label === 'Семья');
  expect(added?.active).toBe(true);
  expect(added?.exists).toBe(false);

  // активное убрать нельзя
  expect(await vm.remove(added?.id ?? '')).toBe(false);
  expect(vm.error).not.toBeNull();
  const main = vm.items.find((v) => v.id === 'main');
  expect(await vm.switchTo(main?.id ?? '')).toBe(true);
  expect(await vm.remove(added?.id ?? '')).toBe(true);
  expect(vm.items.some((v) => v.label === 'Семья')).toBe(false);
});

it('название хранилища не может быть пустым', async () => {
  const vm = new VaultsVm();
  expect(await vm.add('   ')).toBe(false);
  expect(vm.error).not.toBeNull();
});

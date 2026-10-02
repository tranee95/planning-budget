import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { installMocks } from '$lib/api/mock/install';
import { toasts } from '$lib/stores/toasts.svelte';
import { SavingsVm } from './savings.svelte';

beforeEach(() => {
  installMocks();
  toasts.clear();
});

afterEach(() => {
  clearMocks();
});

it('load: коридор 13 / 14 / 15 %', async () => {
  const vm = new SavingsVm();
  await vm.load();
  expect([vm.min, vm.norm, vm.max]).toEqual(['13', '14', '15']);
  expect(vm.loaded).toBe(true);
});

it('save: границы вне порядка не отправляются', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.min = '16';
  await vm.save();
  expect(vm.error).toBe('Нижняя граница не больше нормы, норма не больше верхней границы');
});

it('save: не число — ошибка поля', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.norm = 'abc';
  await vm.save();
  expect(vm.error).toBe('Проценты от 0 до 100, например 13 или 14,5');
});

it('save: корректный коридор сохраняется, ошибки нет', async () => {
  const vm = new SavingsVm();
  await vm.load();
  vm.norm = '14,5';
  await vm.save();
  expect(vm.error).toBeNull();
  expect(toasts.items.at(-1)?.message).toBe('Коридор сбережений сохранён');
});

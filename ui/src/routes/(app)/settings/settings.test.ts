import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockVault } from '$lib/api/mock/handlers';
import { installMocks } from '$lib/api/mock/install';
import { SettingsVm } from './settings.svelte';

beforeEach(() => {
  installMocks();
  resetMockVault();
});

afterEach(() => {
  clearMocks();
});

it('смена пароля недоступна, пока новый пароль короткий или повтор не совпадает', () => {
  const vm = new SettingsVm();
  vm.oldPassword = 'correct horse battery';
  vm.newPassword = 'short';
  vm.repeat = 'short';
  expect(vm.canChange).toBe(false);

  vm.newPassword = 'a much longer password';
  expect(vm.mismatch).toBe(true);
  expect(vm.canChange).toBe(false);

  vm.repeat = 'a much longer password';
  expect(vm.mismatch).toBe(false);
  expect(vm.canChange).toBe(true);
});

it('неверный текущий пароль: ошибка у старого поля, черновик сохраняется', async () => {
  const vm = new SettingsVm();
  vm.oldPassword = 'wrong-password';
  vm.newPassword = vm.repeat = 'a much longer password';
  await vm.changePassword();
  expect(vm.oldError).not.toBeNull();
  expect(vm.changed).toBe(false);
  expect(vm.newPassword).toBe('a much longer password');
  expect(vm.changing).toBe(false);
});

it('успешная смена пароля очищает черновик', async () => {
  const vm = new SettingsVm();
  vm.oldPassword = 'correct horse battery';
  vm.newPassword = vm.repeat = 'a much longer password';
  await vm.changePassword();
  expect(vm.changed).toBe(true);
  expect(vm.oldPassword).toBe('');
  expect(vm.newPassword).toBe('');
});

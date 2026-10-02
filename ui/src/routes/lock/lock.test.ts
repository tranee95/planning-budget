import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it } from 'vitest';
import { resetMockVault } from '$lib/api/mock/handlers';
import { installMocks } from '$lib/api/mock/install';
import { session } from '$lib/stores/session.svelte';
import { LockVm } from './lock.svelte';

beforeEach(() => {
  installMocks();
  resetMockVault();
  session.phase = 'locked';
  session.retryUntil = 0;
  session.recoveryCode = null;
});

afterEach(() => {
  clearMocks();
});

it('экран выбирается по фазе сессии и режиму «забыли пароль»', () => {
  const vm = new LockVm();
  expect(vm.view).toBe('unlock');
  vm.showForgot();
  expect(vm.view).toBe('forgot');
  vm.backToUnlock();
  session.phase = 'setup';
  expect(vm.view).toBe('setup');
  session.recoveryCode = 'K7QM-2XPA';
  expect(vm.view).toBe('recovery');
});

it('разблокировка недоступна с пустым паролем и во время паузы', () => {
  const vm = new LockVm();
  expect(vm.canUnlock).toBe(false);
  vm.password = 'x';
  expect(vm.canUnlock).toBe(true);
  session.retryUntil = Date.now() + 5000;
  expect(vm.waiting).toBe(true);
  expect(vm.canUnlock).toBe(false);
});

it('неверный пароль: ошибка и встряска, фаза остаётся locked', async () => {
  const vm = new LockVm();
  vm.password = 'wrong-password';
  await vm.unlock();
  expect(vm.error).toBe('Неверный пароль.');
  expect(vm.shake).toBe(1);
  expect(session.phase).toBe('locked');
});

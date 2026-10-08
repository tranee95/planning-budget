import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { handlers, resetMockVault } from '../api/mock/handlers';
import { installMocks } from '../api/mock/install';
import { onLock, routeFor, session } from './session.svelte';

beforeEach(() => {
  installMocks();
  resetMockVault();
  session.phase = 'booting';
  session.retryUntil = 0;
  session.recoveryCode = null;
});

afterEach(() => {
  clearMocks();
});

test('boot: хранилище есть и закрыто → locked', async () => {
  await session.boot();
  expect(session.phase).toBe('locked');
});

test('boot: хранилища нет → setup', async () => {
  handlers.vault_reset();
  await session.boot();
  expect(session.phase).toBe('setup');
});

test('unlock с верным паролем открывает сессию', async () => {
  await session.unlock('correct horse battery');
  expect(session.phase).toBe('unlocked');
  expect(session.retryUntil).toBe(0);
});

test('третий неверный пароль включает задержку, верный пароль в окне задержки не принимается', async () => {
  for (let i = 0; i < 3; i++) {
    await expect(session.unlock('wrong-password')).rejects.toMatchObject({
      error: { code: 'WrongPassword' }
    });
  }
  expect(session.retryUntil).toBeGreaterThan(Date.now());
  // Попытка в окне задержки — отдельная ошибка: пароль не проверялся.
  await expect(session.unlock('correct horse battery')).rejects.toMatchObject({
    error: { code: 'TooManyAttempts' }
  });
  expect(session.phase).not.toBe('unlocked');
  expect(session.retryUntil).toBeGreaterThan(Date.now());
});

test('create: код хранится в сторе до подтверждения, потом очищается', async () => {
  handlers.vault_reset();
  await session.create('correct horse battery');
  expect(session.recoveryCode).toMatch(/^[A-Z2-7]{4}(-[A-Z2-7]{2,4})+$/);
  expect(session.phase).toBe('unlocked');

  await session.acknowledgeRecovery();
  expect(session.recoveryCode).toBeNull();
  expect(session.phase).toBe('unlocked');
});

test('unlock: бэкенд выдал новый recovery-код после прерванного перевыпуска → код показывается', async () => {
  const unlock = handlers.vault_unlock;
  handlers.vault_unlock = (() => ({
    recoveryCode: 'NEW1-NEW1-NEW1-NEW1-NEW1-NEW1-NE'
  })) as unknown as typeof unlock;
  try {
    await session.unlock('correct horse battery');
  } finally {
    handlers.vault_unlock = unlock;
  }
  expect(session.phase).toBe('unlocked');
  expect(session.recoveryCode).toBe('NEW1-NEW1-NEW1-NEW1-NEW1-NEW1-NE');
  expect(routeFor(session.phase, '/settings', session.recoveryCode !== null)).toBe('/lock');
});

test('lock вызывает очистку доменных сторов и переводит в locked', async () => {
  const reset = vi.fn();
  onLock(reset);
  await session.unlock('correct horse battery');
  await session.lock();
  expect(reset).toHaveBeenCalled();
  expect(session.phase).toBe('locked');
});

test('Locked из любой команды блокирует сессию', () => {
  const reset = vi.fn();
  onLock(reset);
  session.phase = 'unlocked';
  session.lockedFromBackend();
  expect(reset).toHaveBeenCalled();
  expect(session.phase).toBe('locked');
});

test('блокировка убирает неподтверждённый recovery-код из памяти', async () => {
  handlers.vault_reset();
  await session.create('correct horse battery');
  expect(session.recoveryCode).not.toBeNull();
  session.lockedFromBackend();
  expect(session.recoveryCode).toBeNull();
  expect(session.phase).toBe('locked');
});

test('rekey кладёт новый код в стор, сессия остаётся открытой', async () => {
  await session.unlock('correct horse battery');
  await session.rekey('correct horse battery');
  expect(session.recoveryCode).not.toBeNull();
  expect(session.phase).toBe('unlocked');
  await expect(session.rekey('wrong-password')).rejects.toMatchObject({
    error: { code: 'WrongPassword' }
  });
});

test('afterReset: хранилища нет, код и задержка сброшены', () => {
  session.recoveryCode = 'AAAA';
  session.retryUntil = Date.now() + 5000;
  session.afterReset();
  expect(session.phase).toBe('setup');
  expect(session.recoveryCode).toBeNull();
  expect(session.retryUntil).toBe(0);
});

test.each([
  ['booting', '/', false, null],
  ['locked', '/', false, '/lock'],
  ['locked', '/settings', false, '/lock'],
  ['locked', '/lock', false, null],
  ['setup', '/', false, '/lock'],
  ['unlocked', '/lock', false, '/'],
  ['unlocked', '/lock', true, null],
  ['unlocked', '/settings', true, '/lock'],
  ['unlocked', '/settings', false, null]
] as const)('routeFor(%s, %s, recovery=%s) → %s', (phase, path, pending, expected) => {
  expect(routeFor(phase, path, pending)).toBe(expected);
});

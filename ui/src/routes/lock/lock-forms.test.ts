import { clearMocks } from '@tauri-apps/api/mocks';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test } from 'vitest';
import { installMocks } from '$lib/api/mock/install';
import { resetMockVault } from '$lib/api/mock/handlers';
import { session } from '$lib/stores/session.svelte';
import RecoveryCodeView from './_components/RecoveryCodeView.svelte';
import SetupForm from './_components/SetupForm.svelte';
import UnlockForm from './_components/UnlockForm.svelte';
import { LockVm } from './lock.svelte';

let user: ReturnType<typeof userEvent.setup>;

beforeEach(() => {
  user = userEvent.setup();
  installMocks();
  resetMockVault();
  session.phase = 'locked';
  session.retryUntil = 0;
  session.recoveryCode = null;
});

afterEach(() => {
  clearMocks();
});

async function type(label: string, value: string): Promise<void> {
  const field = screen.getByLabelText(label, { selector: 'input' });
  await user.clear(field);
  await user.click(field);
  await user.paste(value);
}

test('вход: кнопка недоступна, пока поле пустое; неверный пароль показывает ошибку под полем', async () => {
  render(UnlockForm, { vm: new LockVm() });
  const submit = screen.getByRole('button', { name: 'Разблокировать' });
  expect(submit).toBeDisabled();

  await type('Пароль', 'wrong-password');
  expect(submit).toBeEnabled();
  await user.click(submit);

  const alert = await screen.findByRole('alert');
  expect(alert).toHaveTextContent('Неверный пароль.');
  expect(session.phase).toBe('locked');
});

test('вход: верный пароль открывает сессию', async () => {
  render(UnlockForm, { vm: new LockVm() });
  await type('Пароль', 'correct horse battery');
  await user.click(screen.getByRole('button', { name: 'Разблокировать' }));
  await expect.poll(() => session.phase).toBe('unlocked');
});

test('вход: после трёх неудач форма блокируется и показывает обратный отсчёт', async () => {
  render(UnlockForm, { vm: new LockVm() });
  for (let i = 0; i < 3; i++) {
    await type('Пароль', 'wrong-password');
    await user.click(screen.getByRole('button', { name: 'Разблокировать' }));
    await screen.findByRole('alert');
  }
  await expect.poll(() => screen.queryByText(/Следующая попытка через \d+ с/)).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Разблокировать' })).toBeDisabled();
});

test('вход: глаз переключает видимость пароля', async () => {
  render(UnlockForm, { vm: new LockVm() });
  const field = screen.getByLabelText('Пароль', { selector: 'input' });
  expect(field).toHaveAttribute('type', 'password');
  await user.click(screen.getByRole('button', { name: 'Показать пароль' }));
  expect(field).toHaveAttribute('type', 'text');
  expect(screen.getByRole('button', { name: 'Скрыть пароль' })).toBeInTheDocument();
});

test('настройка: создание доступно только при длине ≥ 10 и совпадающих паролях', async () => {
  resetMockVault(false);
  render(SetupForm, { vm: new LockVm() });
  const create = screen.getByRole('button', { name: 'Создать хранилище' });
  expect(create).toBeDisabled();

  await type('Пароль', 'short');
  expect(screen.getByRole('alert')).toHaveTextContent('Не короче 10 символов');

  await type('Пароль', 'correct horse battery');
  await type('Повторите пароль', 'correct horse');
  expect(screen.getByRole('alert')).toHaveTextContent('Пароли не совпадают');
  expect(create).toBeDisabled();

  await type('Повторите пароль', 'correct horse battery');
  expect(create).toBeEnabled();
  await user.click(create);
  await expect.poll(() => session.recoveryCode).not.toBeNull();
});

test('настройка: пароль из списка популярных получает предупреждение, но не блокируется', async () => {
  render(SetupForm, { vm: new LockVm() });
  await type('Пароль', 'qwerty1234');
  expect(screen.getByText(/самых распространённых/)).toBeInTheDocument();
  await type('Повторите пароль', 'qwerty1234');
  expect(screen.getByRole('button', { name: 'Создать хранилище' })).toBeEnabled();
});

test('recovery-код: продолжить можно только после подтверждения; код очищается из стора', async () => {
  session.phase = 'unlocked';
  session.recoveryCode = 'K7QM-2XPA-ZTRB-4HNW-6DVC-ESJF-3Q';
  render(RecoveryCodeView, { code: session.recoveryCode });

  expect(screen.getByLabelText('Ключ восстановления')).toHaveTextContent('K7QM');
  const next = screen.getByRole('button', { name: 'Продолжить' });
  expect(next).toBeDisabled();

  await user.click(screen.getByLabelText('Я сохранил код в надёжном месте'));
  expect(next).toBeEnabled();
  await user.click(next);
  await expect.poll(() => session.recoveryCode).toBeNull();
});

test('recovery-код: скопированный код стирается из буфера при подтверждении', async () => {
  let content = '';
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: (t: string) => {
        content = t;
        return Promise.resolve();
      },
      readText: () => Promise.resolve(content)
    }
  });
  session.phase = 'unlocked';
  session.recoveryCode = 'K7QM-2XPA-ZTRB-4HNW-6DVC-ESJF-3Q';
  render(RecoveryCodeView, { code: session.recoveryCode });

  await user.click(screen.getByRole('button', { name: /Скопировать/ }));
  expect(await screen.findByRole('status')).toHaveTextContent('Буфер очистится через 30 с');
  expect(content).toBe('K7QM-2XPA-ZTRB-4HNW-6DVC-ESJF-3Q');

  await user.click(screen.getByLabelText('Я сохранил код в надёжном месте'));
  await user.click(screen.getByRole('button', { name: 'Продолжить' }));
  await expect.poll(() => content).toBe('');
});

test('recovery-код: сохранение в файл сообщает результат', async () => {
  render(RecoveryCodeView, { code: 'K7QM-2XPA-ZTRB-4HNW-6DVC-ESJF-3Q' });
  await user.click(screen.getByRole('button', { name: /Сохранить в файл/ }));
  expect(await screen.findByRole('status')).toHaveTextContent('Код записан в выбранный файл.');
});

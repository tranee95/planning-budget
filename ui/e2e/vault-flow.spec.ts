import { expect, test } from '@playwright/test';

const PASSWORD = 'correct horse battery';
const RECOVERY_CODE = 'K7QM-2XPA-ZTRB-4HNW-6DVC-ESJF-3Q';

test('холодный старт → пароль → recovery-код → заглушка приложения', async ({ page }) => {
  await test.step('первичная настройка', async () => {
    await page.goto('/?vault=new');
    await expect(page.getByRole('heading', { name: 'Создайте пароль' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Создать хранилище' })).toBeDisabled();
    await page.getByLabel('Пароль', { exact: true }).fill(PASSWORD);
    await page.getByLabel('Повторите пароль').fill(PASSWORD);
    await page.getByRole('button', { name: 'Создать хранилище' }).click();
  });

  await test.step('recovery-код показан и без подтверждения дальше не пускает', async () => {
    await expect(
      page.getByRole('heading', { name: 'Сохраните ключ восстановления' })
    ).toBeVisible();
    await expect(page.getByLabel('Ключ восстановления')).toContainText('K7QM');
    await expect(page.getByRole('button', { name: 'Продолжить' })).toBeDisabled();
    await page.getByLabel('Я сохранил код в надёжном месте').check();
    await page.getByRole('button', { name: 'Продолжить' }).click();
  });

  await test.step('заглушка приложения', async () => {
    await expect(page.getByRole('heading', { name: 'Обзор' })).toBeVisible();
    await expect(page).toHaveURL(/\/$/);
  });
});

test('блокировка по Ctrl+L, неверный пароль, обратный отсчёт и вход', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'С возвращением' })).toBeVisible();

  await test.step('три неверных пароля включают задержку', async () => {
    for (let i = 0; i < 3; i++) {
      await page.getByLabel('Пароль', { exact: true }).fill('wrong-password');
      await page.getByRole('button', { name: 'Разблокировать' }).click();
      await expect(page.getByRole('alert')).toHaveText('Неверный пароль.');
    }
    await expect(page.getByText(/Следующая попытка через \d+ с/)).toBeVisible();
    await expect(page.getByRole('button', { name: 'Разблокировать' })).toBeDisabled();
  });

  await test.step('после задержки верный пароль открывает приложение', async () => {
    await expect(page.getByRole('button', { name: 'Разблокировать' })).toBeDisabled();
    await page.getByLabel('Пароль', { exact: true }).fill(PASSWORD);
    await expect(page.getByRole('button', { name: 'Разблокировать' })).toBeEnabled({
      timeout: 6000
    });
    await page.getByRole('button', { name: 'Разблокировать' }).click();
    await expect(page.getByRole('heading', { name: 'Обзор' })).toBeVisible();
  });

  await test.step('Ctrl+L возвращает на экран входа и закрывает доступ к разделам', async () => {
    await page.keyboard.press('Control+KeyL');
    await expect(page.getByRole('heading', { name: 'С возвращением' })).toBeVisible();
    await page.goto('/settings');
    await expect(page.getByRole('heading', { name: 'С возвращением' })).toBeVisible();
  });
});

test('«Забыли пароль»: ключ восстановления и новый пароль', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Забыли пароль/ }).click();
  await expect(page.getByRole('heading', { name: 'Восстановление доступа' })).toBeVisible();

  await test.step('чужой ключ отклоняется', async () => {
    await page.getByLabel('Ключ восстановления').fill('AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AA');
    await page.getByLabel('Новый пароль', { exact: true }).fill('a completely new secret');
    await page.getByLabel('Повторите новый пароль').fill('a completely new secret');
    await page.getByRole('button', { name: 'Задать новый пароль' }).click();
    await expect(page.getByRole('alert')).toContainText('не подходит к хранилищу');
  });

  await test.step('верный ключ открывает приложение', async () => {
    await page.getByLabel('Ключ восстановления').fill(RECOVERY_CODE.toLowerCase());
    await page.getByRole('button', { name: 'Задать новый пароль' }).click();
    await expect(page.getByRole('heading', { name: 'Обзор' })).toBeVisible();
  });
});

test('смена пароля в настройках', async ({ page }) => {
  await page.goto('/');
  await page.getByLabel('Пароль', { exact: true }).fill(PASSWORD);
  await page.getByRole('button', { name: 'Разблокировать' }).click();
  await page.getByRole('link', { name: 'Настройки' }).click();

  await page.getByLabel('Текущий пароль').fill(PASSWORD);
  await page.getByLabel('Новый пароль', { exact: true }).fill('a completely new secret');
  await page.getByLabel('Повторите новый пароль').fill('a completely new secret');
  await page.getByRole('button', { name: 'Сменить пароль' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Пароль изменён.' })).toBeVisible();
});

test('перевыпуск ключа показывает новый ключ восстановления и возвращает в приложение', async ({
  page
}) => {
  await page.goto('/');
  await page.getByLabel('Пароль', { exact: true }).fill(PASSWORD);
  await page.getByRole('button', { name: 'Разблокировать' }).click();
  await page.getByRole('link', { name: 'Настройки' }).click();

  await page.getByLabel('Пароль для перевыпуска ключа').fill(PASSWORD);
  await page.getByRole('button', { name: 'Перевыпустить ключ' }).click();

  await expect(page.getByRole('heading', { name: 'Сохраните ключ восстановления' })).toBeVisible();
  await page.getByLabel('Я сохранил код в надёжном месте').check();
  await page.getByRole('button', { name: 'Продолжить' }).click();
  await expect(page.getByRole('heading', { name: 'Обзор' })).toBeVisible();
});

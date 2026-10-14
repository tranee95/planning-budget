import { expect, test } from '@playwright/test';

test('обновления: при запуске предлагается новая версия, «Позже» закрывает окно', async ({
  page
}) => {
  await page.goto('/?vault=open&update=1');
  const dialog = page.getByRole('dialog', { name: 'Доступна версия 0.7.0' });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('У вас версия 0.6.16');
  await expect(dialog).toContainText('Исправления и улучшения.');
  await dialog.getByRole('button', { name: 'Позже' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('обновления: последняя версия — окна нет', async ({ page }) => {
  await page.goto('/?vault=open');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('обновления: тумблер в настройках и ручная проверка', async ({ page }) => {
  await page.goto('/settings?vault=open');
  const toggle = page.getByRole('switch', { name: 'Проверять обновления при запуске' });
  await expect(toggle).toBeChecked();
  await page.getByText('Проверять обновления при запуске', { exact: true }).click();
  await expect(toggle).not.toBeChecked();

  await page.getByRole('button', { name: 'Проверить обновления сейчас' }).click();
  await expect(page.getByText('Установлена последняя версия.')).toBeVisible();
});

test('обновления: ручная проверка находит версию, не удалась — сообщение', async ({ page }) => {
  await page.goto('/settings?vault=open&update=1');
  await page.getByRole('button', { name: 'Позже' }).click();
  await page.getByRole('button', { name: 'Проверить обновления сейчас' }).click();
  await expect(page.getByText('Доступна новая версия.')).toBeVisible();

  await page.goto('/settings?vault=open&update=fail');
  await page.getByRole('button', { name: 'Проверить обновления сейчас' }).click();
  await expect(page.getByText('Не удалось проверить обновления.')).toBeVisible();
});

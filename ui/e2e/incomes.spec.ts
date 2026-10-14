import { expect, test } from '@playwright/test';

test('доходы: клик по строке правит доход, крестик удаляет, «Вернуть» возвращает', async ({
  page
}) => {
  await page.goto('/incomes?vault=open');
  const prev = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 12 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await prev.click();
  }
  const row = page.getByRole('listitem').filter({ hasText: 'Подработка' });
  await expect(row).toBeVisible();

  await row.getByRole('button', { name: /Изменить доход/ }).click();
  const dialog = page.getByRole('dialog', { name: 'Доход' });
  await expect(dialog.getByLabel('Источник')).toHaveValue('Подработка');
  await dialog.getByLabel('Источник').fill('Подработка (проект)');
  await dialog.getByLabel('Сумма').fill('12345');
  await page.keyboard.press('Enter');
  await expect(dialog).toBeHidden();
  const edited = page.getByRole('listitem').filter({ hasText: 'Подработка (проект)' });
  await expect(edited).toContainText(/12\s345/);

  await edited.getByRole('button', { name: /Удалить доход/ }).click();
  await expect(edited).toBeHidden();
  await page.getByRole('button', { name: 'Вернуть' }).click();
  await expect(edited).toBeVisible();
});

import { expect, test } from '@playwright/test';

test('настройки: теги — переименование и удаление с подтверждением', async ({ page }) => {
  await page.goto('/settings?vault=open');
  const list = page.getByRole('list', { name: 'Теги' });
  await expect(list.getByText('Подарки')).toBeVisible();

  await test.step('переименование', async () => {
    await page.getByRole('button', { name: 'Переименовать тег «Подарки»' }).click();
    const field = page.getByRole('textbox', { name: 'Название' });
    await field.fill('Праздники');
    await page.keyboard.press('Enter');
    await expect(list.getByText('Праздники')).toBeVisible();
    await expect(list.getByText('Подарки')).toHaveCount(0);
  });

  await test.step('занятое имя отклоняется, окно остаётся', async () => {
    await page.getByRole('button', { name: 'Переименовать тег «Праздники»' }).click();
    await page.getByRole('textbox', { name: 'Название' }).fill('работа');
    await page.keyboard.press('Enter');
    await expect(page.getByText('Тег с таким названием уже есть.')).toBeVisible();
    await page.keyboard.press('Escape');
  });

  await test.step('удаление просит подтверждение, Отмена ничего не меняет', async () => {
    await page.getByRole('button', { name: 'Удалить тег «Работа»' }).click();
    const dialog = page.getByRole('dialog', { name: 'Удалить тег?' });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Отмена' }).click();
    await expect(list.getByText('Работа')).toBeVisible();

    await page.getByRole('button', { name: 'Удалить тег «Работа»' }).click();
    await page
      .getByRole('dialog', { name: 'Удалить тег?' })
      .getByRole('button', { name: 'Удалить' })
      .click();
    await expect(list.getByText('Работа')).toHaveCount(0);
  });
});

test('настройки: после удаления тега поиск #тег больше не находит траты', async ({ page }) => {
  await page.goto('/settings?vault=open');
  const palette = page.getByRole('dialog', { name: 'Поиск и команды' });

  await test.step('до удаления поиск по #Подарки находит трату', async () => {
    await page.getByRole('button', { name: /Поиск и команды/ }).click();
    await palette.locator('input').fill('#подарки');
    await expect(palette.getByText('Лента')).toBeVisible();
    await page.keyboard.press('Escape');
  });

  await test.step('удаление тега, снятого с трат', async () => {
    await page.getByRole('button', { name: 'Удалить тег «Подарки»' }).click();
    const dialog = page.getByRole('dialog', { name: 'Удалить тег?' });
    await expect(dialog).toContainText('Тег стоит на 1 трата');
    await dialog.getByRole('button', { name: 'Удалить' }).click();
    await expect(page.getByRole('dialog', { name: 'Удалить тег?' })).toBeHidden();
  });

  await test.step('после удаления тега поиск пуст', async () => {
    await page.getByRole('button', { name: /Поиск и команды/ }).click();
    await palette.locator('input').fill('#подарки');
    await expect(palette.getByText('Лента')).toBeHidden();
  });
});

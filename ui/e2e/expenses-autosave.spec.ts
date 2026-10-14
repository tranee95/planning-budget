import { expect, test } from '@playwright/test';
import { openSeptember } from './september';

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openSeptember(page);
  await expect(page.locator('article.card').first()).toBeVisible();
});

test('сумма в карточке категории сохраняется кликом в другую область', async ({ page }) => {
  await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
  await page.getByLabel('Наименование, Продукты').fill('Овощи');
  await page.getByLabel('Сумма, ₽').fill('250');
  await page.getByRole('heading', { name: 'Продукты', level: 2 }).click();
  await expect(page.getByLabel('Сумма, ₽')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /Овощи/ })).toBeVisible();
});

test('без наименования строка получает название категории', async ({ page }) => {
  await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
  await page.getByLabel('Сумма, ₽').fill('300');
  await page.getByRole('heading', { name: 'Продукты', level: 2 }).click();
  await expect(page.getByRole('button', { name: /^Продукты\s+300/ })).toBeVisible();
});

test('переход между полями строки не сохраняет, Esc отменяет', async ({ page }) => {
  const rows = page.locator('article.card').first().locator('li');
  const before = await rows.count();
  await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
  await page.getByLabel('Наименование, Продукты').fill('Молоко');
  await page.getByLabel('Наименование, Продукты').press('Tab');
  await expect(page.getByLabel('Сумма, ₽')).toBeFocused();
  await page.getByLabel('Сумма, ₽').fill('80');
  await page.keyboard.press('Escape');
  await expect(page.getByLabel('Сумма, ₽')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /Молоко/ })).toHaveCount(0);
  await expect(rows).toHaveCount(before);
});

test('пустая строка добавления закрывается кликом вне её', async ({ page }) => {
  await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
  await expect(page.getByLabel('Сумма, ₽')).toBeVisible();
  await page.getByRole('heading', { name: 'Продукты', level: 2 }).click();
  await expect(page.getByLabel('Сумма, ₽')).toHaveCount(0);
});

test('правка существующей траты сохраняется кликом вне строки, нетронутая закрывается', async ({
  page
}) => {
  await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
  await page.getByLabel('Наименование, Продукты').fill('Хлеб');
  await page.getByLabel('Сумма, ₽').fill('90');
  await page.keyboard.press('Enter');
  const row = page.getByRole('button', { name: /Хлеб/ });
  await expect(row).toBeVisible();

  await row.click();
  await page.getByLabel('Сумма, ₽').fill('120');
  await page.getByRole('heading', { name: 'Продукты', level: 2 }).click();
  await expect(page.getByRole('button', { name: /Хлеб.*120/ })).toBeVisible();

  await page.getByRole('button', { name: /Хлеб/ }).click();
  await page.getByRole('heading', { name: 'Продукты', level: 2 }).click();
  await expect(page.getByLabel('Сумма, ₽')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /Хлеб.*120/ })).toBeVisible();
});

import { expect, test, type Page } from '@playwright/test';

// UX-7: месяц без трат показывает пустое состояние вместо нулевого итога и пустой сетки.
// В моках данные лежат в сентябре 2026; октябрь получает перенесённый план, и тест удаляет его траты.

async function openEmptyOctober(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/expenses?vault=open');
  const previous = page.getByRole('button', { name: 'Предыдущий месяц' });
  for (let i = 0; i < 36 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await previous.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
  await page.getByRole('button', { name: 'Следующий месяц' }).click();
  await expect(page.getByText('Октябрь 2026')).toBeVisible();
  // Приложение переносит план в новый месяц само; пустой месяц получается, если траты удалить.
  const remove = page.getByRole('button', { name: 'Удалить', exact: true });
  await expect(remove.first()).toBeVisible();
  while ((await remove.count()) > 0) {
    await remove.first().click();
  }
  await expect(page.getByText('В этом месяце трат нет')).toBeVisible();
}

test('блоки: пустой месяц — заголовок и кнопки, итоговой строки нет', async ({ page }) => {
  await openEmptyOctober(page);
  await expect(page.getByText('В этом месяце трат нет')).toBeVisible();
  await expect(page.getByRole('button', { name: /Добавить трату/ })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Скопировать план' })).toBeVisible();
  await expect(page.locator('.totals')).toHaveCount(0);
  await expect(page.locator('article.card')).toHaveCount(0);
});

test('«+ Добавить трату» открывает быстрое добавление', async ({ page }) => {
  await openEmptyOctober(page);
  await page.getByRole('button', { name: /Добавить трату/ }).click();
  await expect(page.getByRole('dialog', { name: 'Новая трата' })).toBeVisible();
});

test('«Скопировать план» переносит траты прошлого месяца и убирает пустое состояние', async ({
  page
}) => {
  await openEmptyOctober(page);
  await page.getByRole('button', { name: 'Скопировать план' }).click();
  await expect(page.getByText('В этом месяце трат нет')).toBeHidden();
  await expect(page.locator('article.card').first()).toBeVisible();
  await expect(page.locator('.totals')).toBeVisible();
});

test('таблица: пустой месяц без фильтра показывает то же пустое состояние', async ({ page }) => {
  await openEmptyOctober(page);
  await page.getByRole('radio', { name: 'Таблица' }).click();
  await expect(page.getByText('В этом месяце трат нет')).toBeVisible();
  await expect(page.getByRole('grid', { name: 'Траты месяца' })).toHaveCount(0);
});

test('сентябрь: итог подписан, пустого состояния нет', async ({ page }) => {
  await openEmptyOctober(page);
  await page.getByRole('button', { name: /Предыдущий месяц/ }).click();
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
  await expect(page.getByText('В этом месяце трат нет')).toBeHidden();
  await expect(page.locator('.totals')).toContainText(/остаток лимитов|сверх лимитов/);
});

test('нет категорий расходов: ссылка на «Категории» не меньше 24 px по высоте', async ({
  page
}) => {
  await page.goto('/expenses?vault=open&fresh=1');
  const link = page.getByRole('link', { name: /Категориях/ });
  await expect(link).toBeVisible();
  const box = await link.boundingBox();
  expect(box?.height ?? 0).toBeGreaterThanOrEqual(24);
  await link.click();
  await expect(page).toHaveURL(/\/categories$/);
});

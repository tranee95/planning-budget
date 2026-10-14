import { expect, test, type Page } from '@playwright/test';

// UX-6: смена статуса и правка суммы дают тост «Отменить», ⌘Z возвращает прежнее значение,
// клик по сумме и по названию ставит курсор в своё поле.

async function openSeptember(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/expenses?vault=open');
  const switcher = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 12 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await switcher.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
}

const rowOf = (page: Page, title: string) => page.getByRole('listitem').filter({ hasText: title });

test('смена статуса показывает тост «Статус: …», «Отменить» возвращает прежний', async ({
  page
}) => {
  await openSeptember(page);
  const row = rowOf(page, 'Кино');
  await row.getByRole('button', { name: /Сменить статус/ }).click();
  await expect(page.getByText('Статус: Незапланировано')).toBeVisible();
  await expect(row.getByRole('button', { name: /^Незапланировано/ })).toBeVisible();

  await page.getByRole('button', { name: 'Отменить', exact: true }).click();
  await expect(row.getByRole('button', { name: /^Долг/ })).toBeVisible();
});

test('серия быстрых кликов по значку оставляет один тост статуса', async ({ page }) => {
  await openSeptember(page);
  const status = rowOf(page, 'Кино').getByRole('button', { name: /Сменить статус/ });
  await status.click();
  await status.click();
  await status.click();
  await expect(page.getByText(/^Статус: /)).toHaveCount(1);
});

test('⌘Z после смены статуса возвращает прежний статус', async ({ page }) => {
  await openSeptember(page);
  const row = rowOf(page, 'Кино');
  await row.getByRole('button', { name: /Сменить статус/ }).click();
  await expect(row.getByRole('button', { name: /^Незапланировано/ })).toBeVisible();
  await page.keyboard.press('Control+z');
  await expect(row.getByRole('button', { name: /^Долг/ })).toBeVisible();
});

test('правка суммы: тост «Сумма изменена», ⌘Z возвращает сумму и итоги месяца', async ({
  page
}) => {
  await openSeptember(page);
  const totals = page.locator('.totals');
  const totalsBefore = await totals.innerText();
  const row = rowOf(page, 'Магазин у дома');
  const sumBefore = await row.locator('.sum').innerText();

  await row.locator('.sum').click();
  const amount = page.getByLabel('Сумма, ₽');
  await expect(amount).toBeFocused();
  await amount.fill('99999');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Сумма изменена')).toBeVisible();
  await expect(row.locator('.sum')).toContainText('99 999');
  await expect(totals).not.toHaveText(totalsBefore);

  await page.keyboard.press('Control+z');
  await expect(row.locator('.sum')).toHaveText(sumBefore);
  await expect(totals).toHaveText(totalsBefore);
});

test('правка суммы: «Отменить» в тосте возвращает сумму', async ({ page }) => {
  await openSeptember(page);
  const row = rowOf(page, 'Магазин у дома');
  const sumBefore = await row.locator('.sum').innerText();
  await row.locator('.sum').click();
  await page.getByLabel('Сумма, ₽').fill('12345');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Сумма изменена')).toBeVisible();
  await page.getByRole('button', { name: 'Отменить', exact: true }).click();
  await expect(row.locator('.sum')).toHaveText(sumBefore);
});

test('правка названия: тост «Строка изменена»', async ({ page }) => {
  await openSeptember(page);
  await rowOf(page, 'Магазин у дома').locator('.name').click();
  const title = page.getByLabel('Наименование', { exact: true });
  await expect(title).toBeFocused();
  await title.fill('Магазин у реки');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Строка изменена')).toBeVisible();
  await expect(page.getByText('Магазин у реки')).toBeVisible();
});

test('Enter на строке без клика открывает правку с курсором в сумме', async ({ page }) => {
  await openSeptember(page);
  const body = rowOf(page, 'Магазин у дома').locator('button.body');
  await body.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByLabel('Сумма, ₽')).toBeFocused();
});

test('таблица: клик по сумме — курсор в сумме, по названию — в названии', async ({ page }) => {
  await openSeptember(page);
  await page.getByRole('radio', { name: 'Таблица' }).click();
  const grid = page.getByRole('grid', { name: 'Траты месяца' });
  const row = grid.getByRole('row', { name: /Магазин у дома/ });

  await row.getByRole('button', { name: /₽/ }).click();
  await expect(page.getByLabel('Сумма, ₽')).toBeFocused();
  await page.keyboard.press('Escape');

  await row.getByRole('button', { name: 'Магазин у дома' }).click();
  await expect(page.getByLabel('Наименование', { exact: true })).toBeFocused();
});

import { expect, test } from '@playwright/test';

// В моках данные лежат в сентябре 2026; приложение открывается на текущем месяце.
async function openSeptember(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/expenses?vault=open');
  const switcher = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 12 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await switcher.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
}

test('расходы: карточки категорий, фильтр статусов, смена статуса, добавление и правка', async ({
  page
}) => {
  await openSeptember(page);

  await test.step('карточки категорий с лимитами', async () => {
    const card = page.locator('article', { has: page.getByRole('heading', { name: 'Продукты' }) });
    await expect(card).toBeVisible();
    await expect(card.getByText('Магазин у дома')).toBeVisible();
    await expect(card.getByRole('progressbar', { name: 'Лимит Продукты' })).toBeVisible();
  });

  await test.step('фильтр по статусу прячет остальные строки', async () => {
    await page.getByRole('button', { name: 'Долг', exact: true }).click();
    await expect(page.getByText('Кино')).toBeVisible();
    await expect(page.getByText('Магазин у дома')).toBeHidden();
    await page.getByRole('button', { name: 'Долг', exact: true }).click();
    await expect(page.getByText('Магазин у дома')).toBeVisible();
  });

  await test.step('клик по значку меняет статус', async () => {
    const row = page.getByRole('listitem').filter({ hasText: 'Кино' });
    await row.getByRole('button', { name: /Сменить статус/ }).click();
    await expect(row.getByRole('button', { name: /^Незапланировано/ })).toBeVisible();
  });

  await test.step('«+» в карточке добавляет трату с клавиатуры', async () => {
    await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
    await page.getByLabel('Наименование, Продукты').fill('Хлеб');
    await page.getByLabel('Сумма, ₽').fill('90');
    await page.keyboard.press('Enter');
    await expect(page.getByText('Хлеб')).toBeVisible();
  });

  await test.step('клик по строке открывает правку, Esc отменяет', async () => {
    await page.getByRole('button', { name: /Хлеб/ }).click();
    const title = page.getByLabel('Наименование', { exact: true });
    await expect(title).toBeFocused();
    await title.fill('Батон');
    await page.keyboard.press('Enter');
    await expect(page.getByText('Батон')).toBeVisible();
    await page.getByRole('button', { name: /Батон/ }).click();
    await page.keyboard.press('Escape');
    await expect(page.getByLabel('Наименование', { exact: true })).toBeHidden();
  });
});

test('расходы → таблица: сортировка, мультивыбор и массовая смена статуса', async ({ page }) => {
  await openSeptember(page);
  await page.getByRole('radio', { name: 'Таблица' }).click();
  const grid = page.getByRole('grid', { name: 'Траты месяца' });
  await expect(grid).toBeVisible();

  await test.step('сортировка по сумме', async () => {
    await grid.getByRole('button', { name: /Сумма/ }).click();
    await expect(grid.getByRole('columnheader', { name: /Сумма/ })).toHaveAttribute(
      'aria-sort',
      'ascending'
    );
    await expect(grid.getByRole('row').nth(1)).toContainText('Интернет');
  });

  await test.step('мультивыбор и массовый статус', async () => {
    await grid.getByLabel('Выбрать «Интернет»').check();
    await grid.getByLabel('Выбрать «Вклад»').check();
    await expect(page.getByText('Выбрано 2')).toBeVisible();
    await page
      .getByRole('toolbar', { name: 'Действия с выбранными' })
      .getByRole('button', { name: 'Оплачено' })
      .click();
    await expect(page.getByText('Выбрано')).toBeHidden();
    await expect(
      grid.getByRole('row', { name: /Интернет/ }).getByRole('button', { name: /^Оплачено/ })
    ).toBeVisible();
  });

  await test.step('клавиатура: активная строка и Пробел', async () => {
    await grid.focus();
    await page.keyboard.press('ArrowDown');
    await expect(grid).toHaveAttribute('aria-activedescendant', /row-1$/);
    await page.keyboard.press('Space');
    await expect(grid.locator('[aria-selected=true]')).toHaveCount(1);
  });
});

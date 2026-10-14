import { expect, test } from '@playwright/test';
import { openQuickAdd } from './quick-add';

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
    await grid.getByLabel('Выбрать «Кино»').check();
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

test('расходы: крестик удаляет добавленную сумму, «Отменить» возвращает', async ({ page }) => {
  await openSeptember(page);
  const row = page.getByRole('listitem').filter({ hasText: 'Магазин у дома' });
  await row.getByRole('button', { name: 'Удалить' }).click();
  await expect(page.getByText('Магазин у дома')).toBeHidden();
  await page.getByRole('button', { name: 'Отменить' }).click();
  await expect(page.getByText('Магазин у дома')).toBeVisible();
});

for (const [width, height] of [
  [1024, 680],
  [1280, 800],
  [1440, 900],
  [1920, 1080]
]) {
  test(`расходы → таблица: панель выбранных не сдвигает таблицу и не ломает раскладку (${String(width)}×${String(height)})`, async ({
    page
  }) => {
    await page.setViewportSize({ width, height });
    await openSeptember(page);
    await page.getByRole('radio', { name: 'Таблица' }).click();
    const grid = page.getByRole('grid', { name: 'Траты месяца' });
    await expect(grid).toBeVisible();
    const before = await grid.boundingBox();

    await grid.getByLabel('Выбрать «Кино»').check();
    const toolbar = page.getByRole('toolbar', { name: 'Действия с выбранными' });
    await expect(toolbar).toBeVisible();

    const after = await grid.boundingBox();
    expect(after?.y).toBe(before?.y);

    const center = async (box: ReturnType<typeof toolbar.boundingBox>): Promise<number> => {
      const b = await box;
      return (b?.y ?? 0) + (b?.height ?? 0) / 2;
    };
    const select = toolbar.getByRole('combobox', { name: 'Категория' });
    const button = toolbar.getByRole('button', { name: 'Удалить' });
    const selectY = await center(select.boundingBox());
    const buttonY = await center(button.boundingBox());
    expect(Math.abs(selectY - buttonY)).toBeLessThanOrEqual(2);

    const box = await toolbar.boundingBox();
    const viewport = page.viewportSize();
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(viewport?.width ?? 0);
  });
}

test('быстрое добавление: сумма → Enter сохраняет в предвыбранную категорию', async ({ page }) => {
  await openQuickAdd(page);
  const dialog = page.getByRole('dialog', { name: 'Новая трата' });
  await expect(dialog.getByLabel('Сумма')).toBeFocused();
  // Категория уже выбрана, наименование не обязательно.
  await expect(
    dialog.getByRole('radiogroup', { name: 'Категория' }).getByRole('radio', { checked: true })
  ).toHaveCount(1);
  await page.keyboard.type('500');
  await page.keyboard.press('Enter');
  await expect(dialog).toBeHidden();
  await expect(page.getByText(/Добавлено: .+, 500\s₽/)).toBeVisible();
});

test('быстрое добавление: Enter без суммы подсказывает, Enter на категории сохраняет', async ({
  page
}) => {
  await openQuickAdd(page);
  const dialog = page.getByRole('dialog', { name: 'Новая трата' });
  await page.keyboard.press('Enter');
  await expect(dialog.getByText('Введите сумму')).toBeVisible();
  await expect(dialog).toBeVisible();

  await dialog.getByLabel('Сумма').fill('700');
  await expect(dialog.getByText('Введите сумму')).toBeHidden();
  await dialog.getByRole('radio', { name: 'Жильё' }).focus();
  await page.keyboard.press('Enter');
  await expect(dialog).toBeHidden();
  await expect(page.getByText(/Добавлено: Жильё, 700\s₽/)).toBeVisible();
});

test('быстрое добавление: ⇧↵ оставляет форму и ставит курсор в сумму', async ({ page }) => {
  await openQuickAdd(page);
  const dialog = page.getByRole('dialog', { name: 'Новая трата' });
  await page.keyboard.type('100');
  await page.keyboard.press('Shift+Enter');
  await expect(page.getByText(/Добавлено: .+, 100\s₽/)).toBeVisible();
  await expect(dialog.getByLabel('Сумма')).toBeFocused();
  await page.keyboard.type('250');
  await expect(dialog.getByLabel('Сумма')).toHaveValue(/250/);
  await expect(dialog.getByLabel('Наименование')).toHaveValue('');
});

test('быстрое добавление: кнопка «Сохранить» без суммы не отключена и объясняет, чего не хватает', async ({
  page
}) => {
  await openQuickAdd(page);
  const dialog = page.getByRole('dialog', { name: 'Новая трата' });
  const save = dialog.getByRole('button', { name: /Сохранить/ });
  await expect(save).toBeEnabled();
  await save.click();
  await expect(dialog.getByText('Введите сумму')).toBeVisible();
  await expect(dialog).toBeVisible();
});

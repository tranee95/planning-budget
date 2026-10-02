import { expect, test, type Page } from '@playwright/test';

// В моках данные лежат в сентябре 2026, история «Ленты» — в 2024.
async function openShell(page: Page, path = '/expenses'): Promise<void> {
  await page.goto(`${path}?vault=open`);
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
}

async function openSeptember(page: Page): Promise<void> {
  await openShell(page);
  const back = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 12 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await back.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
}

test('E2E 4: ⌘K «лента статус:оплачено» → 7 трат на 90 681 ₽, Enter открывает трату на правку', async ({
  page
}) => {
  await openShell(page, '/');
  await page.keyboard.press('Control+KeyK');
  await page.keyboard.type('лента статус:оплачено');

  const group = page.getByRole('group', { name: /^Траты · найдено 7 · 90\s681\s₽$/ });
  await expect(group).toBeVisible();
  await expect(group.getByRole('option')).toHaveCount(7);

  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/expenses$/);
  await expect(page.getByRole('textbox', { name: 'Наименование' })).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Наименование' })).toHaveValue('Лента');
});

test('палитра: Ctrl+Enter открывает все найденные траты таблицей с запросом', async ({ page }) => {
  await openShell(page, '/');
  await page.keyboard.press('Control+KeyK');
  await page.keyboard.type('лента статус:оплачено');
  await expect(page.getByRole('group', { name: /^Траты/ })).toBeVisible();
  await page.keyboard.press('Control+Enter');

  await expect(page).toHaveURL(/\/expenses$/);
  await expect(page.getByRole('textbox', { name: 'Фильтр трат' })).toHaveValue(
    'лента статус:оплачено'
  );
  await expect(page.getByRole('status').filter({ hasText: 'Найдено трат' })).toContainText(
    /7 на 90\s681\s₽/
  );
  await expect(page.getByRole('row', { name: /Лента/ })).toHaveCount(7);
});

test('панель фильтров: чипы, строка итога, сохранение и применение фильтра', async ({ page }) => {
  await openShell(page);
  await page.getByRole('radio', { name: 'Таблица' }).click();
  const field = page.getByRole('textbox', { name: 'Фильтр трат' });

  await field.fill('лента статус:оплачено');
  await expect(page.getByText('текст: «лента»')).toBeVisible();
  await expect(page.getByText('статус:оплачено')).toBeVisible();
  await expect(page.getByRole('status').filter({ hasText: 'Найдено трат' })).toContainText(
    /7 на 90\s681\s₽/
  );
  await expect(page.getByRole('columnheader', { name: /Месяц/ })).toBeVisible();

  await page.getByRole('button', { name: 'Убрать статус статус:оплачено' }).click();
  await expect(field).toHaveValue('лента');
  await expect(page.getByRole('status').filter({ hasText: 'Найдено трат' })).toContainText(/8 на/);

  await page.getByRole('button', { name: 'Сохранить фильтр' }).click();
  await page.getByRole('textbox', { name: 'Название фильтра' }).fill('Лента');
  await page.keyboard.press('Enter');
  const saved = page.getByRole('list', { name: 'Сохранённые фильтры' });
  await expect(saved.getByRole('button', { name: 'Лента', exact: true })).toBeVisible();

  await page.getByRole('button', { name: 'Очистить фильтр' }).click();
  await expect(page.getByRole('columnheader', { name: /Месяц/ })).toBeHidden();
  await saved.getByRole('button', { name: 'Лента', exact: true }).click();
  await expect(field).toHaveValue('лента');
});

test('мягкая подсказка: неизвестный статус уходит в текст и объясняется', async ({ page }) => {
  await openShell(page);
  await page.getByRole('radio', { name: 'Таблица' }).click();
  await page.getByRole('textbox', { name: 'Фильтр трат' }).fill('статус:абракадабра');
  await expect(page.getByText(/такого значения нет/)).toBeVisible();
  await expect(page.getByText('Ничего не найдено').first()).toBeVisible();
});

test('теги: выбор тегов в таблице, поиск по #тегу', async ({ page }) => {
  await openSeptember(page);
  await page.getByRole('radio', { name: 'Таблица' }).click();
  await page
    .getByRole('button', { name: /^Теги: нет\. Изменить/ })
    .first()
    .click();
  const dialog = page.getByRole('dialog', { name: /^Теги:/ });
  await dialog.getByRole('checkbox', { name: 'Работа' }).check();
  await dialog.getByRole('button', { name: 'Сохранить' }).click();
  await expect(page.getByRole('button', { name: /^Теги: Работа\. Изменить/ })).toBeVisible();

  await page.getByRole('textbox', { name: 'Фильтр трат' }).fill('#работа');
  await expect(page.getByRole('status').filter({ hasText: 'Найдено трат' })).toContainText(/1 на/);
});

test('доходы: панель фильтров ищет по всем месяцам', async ({ page }) => {
  await openShell(page, '/incomes');
  await page.getByRole('textbox', { name: 'Фильтр доходов' }).fill('подработка');
  await expect(page.getByRole('status').filter({ hasText: 'Найдено доходов' })).toContainText(
    /1 на/
  );
  await expect(
    page.getByRole('list', { name: 'Доходы месяца' }).getByText('Подработка')
  ).toBeVisible();
});

test('быстрое добавление: подсказки из всей истории', async ({ page }) => {
  await openShell(page);
  await page.keyboard.press('KeyN');
  await page.getByLabel('Наименование').fill('лен');
  await expect(page.getByRole('list', { name: 'Подсказки из истории' })).toContainText('Лента');
});

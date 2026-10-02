import { expect, test, type Page } from '@playwright/test';

// В моках данные лежат в сентябре 2026; приложение открывается на текущем месяце.
async function openSeptember(page: Page, path: string): Promise<void> {
  await page.goto(`${path}?vault=open`);
  const prev = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 24 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await prev.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
}

test('категории: новая категория с лимитом с октября появляется в блоках октября', async ({
  page
}) => {
  await openSeptember(page, '/categories');
  await page.getByRole('button', { name: /Следующий месяц/ }).click();
  await expect(page.getByText('Октябрь 2026')).toBeVisible();

  await test.step('создание с лимитом', async () => {
    await page.getByRole('button', { name: 'Категория' }).click();
    await page.getByLabel('Название').fill('Путешествия');
    await page.getByLabel('Лимит в месяц').fill('10000');
    await page.getByRole('button', { name: 'Сохранить' }).click();
    await expect(page.getByRole('button', { name: /Путешествия/ })).toBeVisible();
    await expect(page.getByText('Действует с Октябрь 2026')).toBeVisible();
  });

  await test.step('в сентябре у категории нет лимита', async () => {
    await page.getByRole('button', { name: /Предыдущий месяц/ }).click();
    const row = page.getByRole('button', { name: /Путешествия/ });
    await expect(row).toBeVisible();
    await expect(row.getByText('—').first()).toBeVisible();
  });

  await test.step('в блоках октября у карточки есть лимит', async () => {
    await page.getByRole('link', { name: 'Расходы' }).click();
    await page.getByRole('button', { name: /Следующий месяц/ }).click();
    await expect(page.getByRole('progressbar', { name: 'Лимит Путешествия' })).toBeVisible();
  });
});

test('категории: перестановка клавиатурой и архив', async ({ page }) => {
  await openSeptember(page, '/categories');
  const rows = page.getByRole('button', { name: /, (Обязательные|Желания|Сбережения)/ });
  await expect(rows.first()).toContainText('Продукты');
  await rows.first().focus();
  await page.keyboard.press('Alt+ArrowDown');
  await expect(rows.first()).toContainText('Жильё');

  await page.getByRole('button', { name: /Развлечения/ }).click();
  await page.getByRole('button', { name: 'В архив' }).click();
  await expect(page.getByRole('button', { name: /Развлечения/ })).toBeHidden();
});

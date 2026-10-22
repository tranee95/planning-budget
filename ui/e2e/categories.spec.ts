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
    await page.getByRole('button', { name: 'Добавить категорию' }).click();
    await page.getByLabel('Название').fill('Путешествия');
    await page.getByLabel('Лимит в месяц').fill('10000');
    await page.getByRole('button', { name: 'Сохранить' }).click();
    await expect(page.getByRole('button', { name: /Путешествия/ })).toBeVisible();
    await expect(page.getByText('Сохранено: Путешествия')).toBeVisible();
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

test('категории: архив из редактора', async ({ page }) => {
  await openSeptember(page, '/categories');
  await page.getByRole('button', { name: /Развлечения/ }).click();
  await page
    .getByRole('dialog', { name: 'Развлечения' })
    .getByRole('button', { name: 'В архив', exact: true })
    .click();
  // Архивация требует подтверждения: до него категория остаётся в списке.
  const dialog = page.getByRole('dialog', { name: 'Отправить в архив?' });
  await expect(dialog).toBeVisible();
  await expect(page.getByRole('button', { name: /Развлечения, / })).toBeVisible();
  await dialog.getByRole('button', { name: 'В архив' }).click();
  await expect(page.getByRole('button', { name: /Развлечения, / })).toBeHidden();
});

test('категории: кнопка архива в строке спрашивает подтверждение, «Отмена» ничего не меняет', async ({
  page
}) => {
  await openSeptember(page, '/categories');
  const row = page.getByRole('listitem').filter({ hasText: 'Развлечения' });
  await row.getByRole('button', { name: 'В архив' }).click();

  const dialog = page.getByRole('dialog', { name: 'Отправить в архив?' });
  await expect(dialog).toContainText('Развлечения');
  await dialog.getByRole('button', { name: 'Отмена' }).click();
  await expect(dialog).toBeHidden();
  await expect(row).toBeVisible();

  await row.getByRole('button', { name: 'В архив' }).click();
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await expect(row).toBeVisible();

  await row.getByRole('button', { name: 'В архив' }).click();
  await dialog.getByRole('button', { name: 'В архив' }).click();
  await expect(row).toBeHidden();

  await page.getByText('Показать архив').click();
  const archived = page.getByRole('listitem').filter({ hasText: 'Развлечения' });
  await expect(archived).toBeVisible();
  await archived.getByRole('button', { name: 'Вернуть из архива' }).click();
  await expect(archived.getByRole('button', { name: 'В архив' })).toBeVisible();
});

const names = (page: Page) => page.locator('li.item .name').allTextContents();

async function drag(
  page: Page,
  from: number,
  to: number,
  finish: 'drop' | 'escape'
): Promise<void> {
  const items = page.locator('li.item');
  const grip = items.nth(from).getByRole('button', { name: 'Перетащить' });
  const target = await items.nth(to).boundingBox();
  const start = await grip.boundingBox();
  if (!start || !target) throw new Error('нет раскладки');
  await page.mouse.move(start.x + start.width / 2, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(start.x + 10, start.y + 10, { steps: 4 });
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 8 });
  if (finish === 'escape') await page.keyboard.press('Escape');
  await page.mouse.up();
}

test('категории: строку можно перетащить за ручку, порядок сохраняется', async ({ page }) => {
  await openSeptember(page, '/categories');
  expect(await names(page)).toEqual(['Продукты', 'Жильё', 'Развлечения']);
  await drag(page, 0, 2, 'drop');
  await expect.poll(() => names(page)).toEqual(['Жильё', 'Развлечения', 'Продукты']);
  await drag(page, 2, 0, 'drop');
  await expect.poll(() => names(page)).toEqual(['Продукты', 'Жильё', 'Развлечения']);
});

test('категории: Escape во время перетаскивания отменяет перенос', async ({ page }) => {
  await openSeptember(page, '/categories');
  await drag(page, 0, 2, 'escape');
  expect(await names(page)).toEqual(['Продукты', 'Жильё', 'Развлечения']);
  await expect(page.locator('li.lifted, li.target')).toHaveCount(0);
});

test('категории: перетаскивание не открывает редактор, а Alt+↑/↓ порядок не меняет', async ({
  page
}) => {
  await openSeptember(page, '/categories');
  await drag(page, 0, 1, 'drop');
  await expect(page.getByRole('button', { name: 'Сохранить' })).toHaveCount(0);
  const before = await names(page);
  await page.getByRole('button', { name: /^Жильё, / }).focus();
  await page.keyboard.press('Alt+ArrowDown');
  expect(await names(page)).toEqual(before);
});

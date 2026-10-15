import { expect, test, type Page } from '@playwright/test';
import { addAmount, addGoal, pickMenu } from './savings-helpers';

async function addDeposit(page: Page, name: string): Promise<void> {
  const card = page.getByRole('article', { name });
  await addAmount(card, '3000');
  await expect(card.locator('header')).toContainText(/3\s000/);
}

test('#137: клик по названию, ввод, Enter: имя обновляется в карточке и в легенде графика', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  await addGoal(page, 'Отпуск');
  await addDeposit(page, 'Накопления');
  await addDeposit(page, 'Отпуск');
  const legend = page.locator('figure .legend');
  await expect(legend).toContainText('Накопления');

  await page.getByRole('button', { name: 'Переименовать накопление «Накопления»' }).click();
  const field = page.getByLabel('Название накопления');
  await expect(field).toBeFocused();
  await expect(field).toHaveValue('Накопления');
  await page.keyboard.type('Подушка');
  await page.keyboard.press('Enter');

  await expect(field).toHaveCount(0);
  await expect(page.getByRole('article', { name: 'Подушка' })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Переименовать накопление «Подушка»' })
  ).toBeVisible();
  await expect(legend).toContainText('Подушка');
  await expect(legend).not.toContainText('Накопления');
  await expect(page.getByText('Переименовано: «Подушка»')).toBeVisible();

  // «Отменить» в тосте возвращает прежнее имя.
  await page.getByRole('button', { name: 'Отменить' }).click();
  await expect(page.getByRole('article', { name: 'Накопления' })).toBeVisible();
  await expect(legend).toContainText('Накопления');
});

test('#137: Esc отменяет правку, потеря фокуса сохраняет изменённое', async ({ page }) => {
  await page.goto('/savings?vault=open');
  await page.getByRole('button', { name: 'Переименовать накопление «Накопления»' }).click();
  await page.keyboard.type('Черновик');
  await page.keyboard.press('Escape');
  await expect(page.getByLabel('Название накопления')).toHaveCount(0);
  await expect(page.getByRole('article', { name: 'Накопления' })).toBeVisible();

  await page.getByRole('button', { name: 'Переименовать накопление «Накопления»' }).click();
  await page.keyboard.type('Резерв');
  await page.getByRole('heading', { level: 1 }).click();
  await expect(page.getByRole('article', { name: 'Резерв' })).toBeVisible();

  // Нетронутое имя при потере фокуса просто закрывает поле: новых тостов нет.
  await expect(page.getByText(/^Переименовано:/)).toHaveCount(1);
  await page.getByRole('button', { name: 'Переименовать накопление «Резерв»' }).click();
  await page.getByRole('heading', { level: 1 }).click();
  await expect(page.getByLabel('Название накопления')).toHaveCount(0);
  await expect(page.getByText(/^Переименовано:/)).toHaveCount(1);
});

test('#137: занятое имя и пустое имя отклоняются тостом, прежнее имя остаётся', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  await addGoal(page, 'Отпуск');
  await expect(page.getByRole('article', { name: 'Отпуск' })).toBeVisible();

  await page.getByRole('button', { name: 'Переименовать накопление «Накопления»' }).click();
  await page.keyboard.type('отпуск');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Категория с таким названием уже есть.')).toBeVisible();
  await expect(page.getByRole('article', { name: 'Накопления' })).toBeVisible();

  await page.getByRole('button', { name: 'Переименовать накопление «Накопления»' }).click();
  await page.keyboard.press('Delete');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Укажите название категории.')).toBeVisible();
  await expect(page.getByRole('article', { name: 'Накопления' })).toBeVisible();
});

test('#137: у закрытой цели нет кнопки переименования', async ({ page }) => {
  await page.goto('/savings?vault=open');
  await addGoal(page, 'Отпуск');
  await pickMenu(page.getByRole('article', { name: 'Отпуск' }), 'Закрыть цель');
  await page
    .getByRole('dialog', { name: 'Закрыть цель?' })
    .getByRole('button', { name: 'Закрыть цель' })
    .click();
  await page.getByRole('button', { name: /Закрытые цели/ }).click();
  await expect(page.getByText('Отпуск').first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Переименовать накопление «Отпуск»' })).toHaveCount(
    0
  );
});

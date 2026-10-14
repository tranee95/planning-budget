import { expect, test } from '@playwright/test';
import { openSeptember } from './september';

test('UX-5: при открытии строки суммы фокус в поле суммы, название второе', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: /Добавить сумму/ }).click();
  await expect(page.getByLabel('Сумма, ₽')).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(page.getByLabel('Наименование, Накопления')).toBeFocused();
});

test('UX-5: три суммы подряд без кнопки: Enter сохраняет и оставляет курсор в сумме', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: /Добавить сумму/ }).click();
  for (let i = 0; i < 3; i++) {
    await page.keyboard.type('10000');
    await page.keyboard.press('Enter');
    await expect(card.getByRole('listitem').filter({ hasText: /10\s000/ })).toHaveCount(i + 1);
    await expect(page.getByLabel('Сумма, ₽')).toBeFocused();
    await expect(page.getByLabel('Сумма, ₽')).toHaveValue('');
  }
  await expect(card.locator('header')).toContainText(/30\s000/);
  await page.keyboard.press('Escape');
  await expect(page.getByLabel('Сумма, ₽')).toHaveCount(0);
  await expect(card.locator('header')).toContainText(/30\s000/);
});

test('UX-5: «Расходы» сохраняют прежнее поведение: Enter закрывает строку ввода', async ({
  page
}) => {
  await openSeptember(page);
  const card = page.getByRole('article').filter({ hasText: 'Продукты' }).first();
  await card
    .getByRole('button', { name: /Добавить/ })
    .first()
    .click();
  await page.getByLabel('Сумма, ₽').fill('150');
  await page.keyboard.press('Enter');
  await expect(page.getByLabel('Сумма, ₽')).toHaveCount(0);
});

test('UX-5: месяц в предложном падеже', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await expect(card.locator('header')).toContainText(/Отложено в [а-я]+е \d{4}:/);
  await expect(card.locator('header')).not.toContainText(/Отложено в [А-Я]/);
  await page.getByRole('button', { name: /Предыдущий месяц/ }).click();
  await expect(card).toContainText(/В [а-я]+е \d{4} ещё ничего не откладывали/);
  await expect(card).not.toContainText(/В [А-Я][а-я]+ \d{4} ещё/);
});

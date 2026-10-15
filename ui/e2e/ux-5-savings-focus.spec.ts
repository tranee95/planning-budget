import { expect, test } from '@playwright/test';
import { records } from './savings-helpers';
import { openSeptember } from './september';

test('UX-5: строка ввода всегда на месте, порядок Tab: комментарий → сумма → «+»', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByLabel('Комментарий').focus();
  await page.keyboard.press('Tab');
  await expect(card.getByLabel('Сумма, ₽')).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(card.getByRole('button', { name: 'Добавить сумму' })).toBeFocused();
});

test('UX-5: три суммы подряд без кнопки: Enter сохраняет и оставляет курсор в сумме', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByLabel('Сумма, ₽').focus();
  for (let i = 0; i < 3; i++) {
    await page.keyboard.type('10000');
    await page.keyboard.press('Enter');
    await expect(records(card).filter({ hasText: /10\s000/ })).toHaveCount(i + 1);
    await expect(card.getByLabel('Сумма, ₽')).toBeFocused();
    await expect(card.getByLabel('Сумма, ₽')).toHaveValue('');
  }
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
  await expect(card).toContainText(/Пока ничего не отложено в [а-я]+е \d{4}/);
  await expect(card).not.toContainText(/Пока ничего не отложено в [А-Я]/);
  await page.getByRole('button', { name: /Предыдущий месяц/ }).click();
  await expect(card).toContainText(/ничего не отложено в [а-я]+е \d{4}/);
  await expect(card).not.toContainText(/отложено в [А-Я][а-я]+ \d{4}/);
});

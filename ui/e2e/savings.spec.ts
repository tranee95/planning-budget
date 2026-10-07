import { expect, test } from '@playwright/test';

test('сбережения: итоги, параметры накопления, факт, прогноз с дисклеймером и заглушка брокера', async ({
  page
}) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  await page.goto('/savings?vault=open');
  await expect(page.getByRole('heading', { level: 1, name: 'Сбережения' })).toBeVisible();

  await expect(page.getByText('Баланс накоплений')).toBeVisible();
  await expect(page.getByLabel('Ставка, % годовых')).toHaveValue('16');
  await expect(page.getByText(/С учётом налога: 13,92\s%/)).toBeVisible();

  const fact = page.getByRole('table', { name: /Накопление по месяцам/ });
  await expect(fact.getByRole('row')).toHaveCount(13);
  await expect(page.getByRole('img', { name: 'Баланс накопления по месяцам' })).toBeVisible();

  const forecast = page.getByRole('table', { name: 'Итоги прогноза по сценариям' });
  await expect(forecast.getByRole('row')).toHaveCount(4);
  await expect(forecast.getByText('A · цель нормы')).toBeVisible();
  await expect(page.getByText(/Ставка и выплаты не гарантированы/)).toBeVisible();

  await expect(page.getByRole('button', { name: 'Подключить брокерский счёт' })).toBeDisabled();
  expect(problems).toEqual([]);
});

test('сбережения: сохранение параметров и ошибка ввода рядом с кнопкой', async ({ page }) => {
  await page.goto('/savings?vault=open');
  await page.getByLabel('Ставка, % годовых').fill('шестнадцать');
  await page.getByRole('button', { name: 'Сохранить' }).click();
  await expect(page.getByRole('alert')).toContainText('от 0 до 100');

  await page.getByLabel('Ставка, % годовых').fill('17');
  await page.getByRole('button', { name: 'Сохранить' }).click();
  await expect(page.getByText('Накопление сохранено')).toBeVisible();
});

test('сбережения: план суммой вместо процента', async ({ page }) => {
  await page.goto('/savings?vault=open');
  await page.getByRole('radio', { name: 'Сумма' }).click();
  await page.getByLabel('Сумма в месяц').fill('5000');
  await page.getByRole('button', { name: 'Сохранить' }).click();
  await expect(page.getByText('Накопление сохранено')).toBeVisible();
  await expect(page.getByText(/5\s000\s₽ в месяц/)).toBeVisible();
});

test('сбережения: клавиша 6 ведёт на раздел, а /bonds перенаправляет на него', async ({ page }) => {
  await page.goto('/?vault=open');
  await page.locator('body').click({ position: { x: 700, y: 400 } });
  await page.keyboard.press('Digit6');
  await expect(page).toHaveURL(/\/savings$/);

  await page.goto('/bonds?vault=open');
  await expect(page).toHaveURL(/\/savings$/);
});

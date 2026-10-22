import { expect, test, type Page } from '@playwright/test';
import { openSeptember } from './september';

// UX-4: после мастера плановые строки не считаются «Фактически»; оплаченная трата считается.
async function planWithFood(page: Page): Promise<void> {
  await page.goto('/?vault=open&intro=1&fresh=1');
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();
  await page
    .getByRole('dialog', { name: 'Категории расходов' })
    .getByRole('button', { name: 'Далее' })
    .click();
  await page.getByRole('button', { name: 'Спланировать первый месяц' }).click();
  const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
  await wizard.getByLabel('Сумма').fill('100000');
  await wizard.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('dialog').getByLabel('Продукты', { exact: true }).fill('30000');
  await page.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('button', { name: 'Далее' }).click();
  await page
    .getByRole('dialog', { name: /: Итог/ })
    .getByRole('button', { name: 'План готов' })
    .click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
}

function foodCells(page: Page) {
  return page.getByRole('row', { name: /Продукты/ }).getByRole('cell');
}

test('«Фактически» на обзоре: 0 ₽ после мастера, сумма после оплаты', async ({ page }) => {
  await planWithFood(page);

  await expect(page.getByRole('columnheader', { name: 'Фактически' })).toBeVisible();
  await expect(page.getByRole('columnheader', { name: 'Факт + план' })).toBeVisible();
  await expect(foodCells(page).nth(0)).toHaveText(/^30\s000\s₽/);
  await expect(foodCells(page).nth(1)).toHaveText(/^0\s₽/);
  await expect(foodCells(page).nth(2)).toHaveText(/^30\s000\s₽/);

  await page.getByRole('link', { name: 'Расходы' }).click();
  await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
  await page.getByLabel('Наименование, Продукты').fill('Рынок');
  await page.getByLabel('Сумма, ₽').fill('1200');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('listitem').filter({ hasText: 'Рынок' })).toBeVisible();

  await page.getByRole('link', { name: 'Обзор' }).click();
  await expect(foodCells(page).nth(1)).toHaveText(/^1\s200\s₽/);
  await expect(foodCells(page).nth(2)).toHaveText(/^31\s200\s₽/);
});

test('KPI «Свободный остаток» и «На неделю» подписаны «с учётом плана»', async ({ page }) => {
  await page.goto('/?vault=open');
  await page.getByRole('heading', { level: 1, name: 'Обзор' }).waitFor();
  await expect(page.getByText('после трат и сбережений, с учётом плана')).toBeVisible();
  await expect(page.getByText('остаток ÷ недели, с учётом плана')).toBeVisible();
});

test('под «Свободным остатком» строка «по факту», у «На неделю» — оставшиеся недели', async ({
  page
}) => {
  await page.goto('/?vault=open');
  await page.getByRole('heading', { level: 1, name: 'Обзор' }).waitFor();
  await expect(page.getByText(/^по факту: /)).toBeVisible();
  // Мок открывает месяц «сегодня», поэтому показаны оставшиеся недели месяца.
  await expect(page.getByText(/^до конца месяца осталось \d+,\d недели$/)).toBeVisible();
});

test('«Отложено» на обзоре — оплаченное, запланированное отдельно («из плана»)', async ({
  page
}) => {
  await openSeptember(page);
  await page.getByRole('link', { name: 'Обзор' }).click();
  const saved = page.locator('dl.summary > div', {
    has: page.locator('dt', { hasText: 'Отложено' })
  });
  await expect(saved).toBeVisible();
  await expect(saved).toContainText(/из плана [\d\s]+₽/);
});

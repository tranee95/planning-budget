import { expect, test } from '@playwright/test';

test('справка: «Как пользоваться» и «Горячие клавиши»', async ({ page }) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  await page.goto('/help?vault=open');
  await expect(page.getByRole('heading', { level: 1, name: 'Справка' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Цикл месяца' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Как спланировать месяц' })).toBeVisible();

  await page.getByRole('tab', { name: 'Горячие клавиши' }).click();
  await expect(page.getByText('поиск и команды', { exact: true })).toBeVisible();
  await expect(page.getByText('новая трата')).toBeVisible();
  expect(problems).toEqual([]);
});

test('справка: клавиша «?» открывает её с любого экрана', async ({ page }) => {
  await page.goto('/?vault=open');
  await page.locator('body').click({ position: { x: 700, y: 400 } });
  await page.keyboard.press('Shift+Slash');
  await expect(page).toHaveURL(/\/help$/);
});

test('справка: команда палитры «Как пользоваться» и мастер из справки', async ({ page }) => {
  await page.goto('/?vault=open');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await page.keyboard.press('Control+KeyK');
  await page.keyboard.type('как пользоваться');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/help$/);

  await page.getByRole('button', { name: 'Спланировать месяц' }).click();
  await expect(page.getByRole('dialog', { name: /План на .+: Доход/ })).toBeVisible();
});

test('справка: пункт меню виден и подсвечен на своей странице', async ({ page }) => {
  await page.goto('/help?vault=open');
  await expect(page.getByRole('link', { name: 'Справка' })).toHaveAttribute('aria-current', 'page');
});

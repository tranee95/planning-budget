import { expect, test } from '@playwright/test';

// Владелец убрал глобальное создание записей: кнопку «+ Трата» в верхней панели, клавиши N и I
// и команды ⌘K «Новая трата» / «Новый доход». Создание остаётся кнопками на экранах.

test('клавиши N и I ничего не открывают', async ({ page }) => {
  for (const screen of ['/', '/expenses', '/incomes', '/savings']) {
    await page.goto(`${screen}?vault=open`);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await page.locator('body').click({ position: { x: 700, y: 500 } });
    const url = page.url();
    await page.keyboard.press('n');
    await page.keyboard.press('i');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    expect(page.url()).toBe(url);
  }
});

test('палитра ⌘K без команд «Новая трата» и «Новый доход»', async ({ page }) => {
  await page.goto('/?vault=open');
  await page.getByRole('button', { name: /Поиск и команды/ }).click();
  await expect(page.getByRole('option', { name: /Обзор/ })).toBeVisible();
  await expect(page.getByRole('option', { name: /Новая трата/ })).toHaveCount(0);
  await expect(page.getByRole('option', { name: /Новый доход/ })).toHaveCount(0);
});

test('в верхней панели нет кнопки «+ Трата»', async ({ page }) => {
  for (const screen of ['/', '/expenses', '/incomes', '/settings']) {
    await page.goto(`${screen}?vault=open`);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.getByRole('banner').getByRole('button', { name: 'Новая трата' })).toHaveCount(
      0
    );
  }
});

test('узкое окно: поиск остаётся в окне, без горизонтальной прокрутки', async ({ page }) => {
  await page.setViewportSize({ width: 960, height: 640 });
  await page.goto('/expenses?vault=open');
  const search = page.getByRole('button', { name: /Поиск и команды/ });
  await expect(search).toBeVisible();
  const b = await search.boundingBox();
  expect((b?.x ?? 0) + (b?.width ?? 0)).toBeLessThanOrEqual(960);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

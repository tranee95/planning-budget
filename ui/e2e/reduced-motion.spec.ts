import { expect, test } from '@playwright/test';

// «Меньше анимаций» и prefers-reduced-motion обнуляют движение: токены длительностей равны 0.
const DURATIONS = [
  '--dur-fast',
  '--dur-base',
  '--dur-slow',
  '--dur-pop',
  '--dur-drawer',
  '--dur-shake'
];

async function durations(page: import('@playwright/test').Page): Promise<number[]> {
  // стили подключаются вместе с приложением: без этого токены ещё пусты
  await page.getByRole('heading', { level: 1 }).waitFor();
  return page.evaluate((names) => {
    const style = getComputedStyle(document.documentElement);
    return names.map((n) => Number.parseFloat(style.getPropertyValue(n)) || 0);
  }, DURATIONS);
}

test('prefers-reduced-motion обнуляет токены длительностей', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/?vault=open');
  expect(await durations(page)).toEqual(DURATIONS.map(() => 0));
});

test.describe('без reduce', () => {
  test.use({ reducedMotion: 'no-preference' });
  test('длительности ненулевые', async ({ page }) => {
    await page.goto('/?vault=open');
    const values = await durations(page);
    expect(values).toEqual([120, 200, 320, 340, 280, 300]);
  });
});

test('при reduce новая строка траты не получает анимаций', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/expenses?vault=open');
  const previous = page.getByRole('button', { name: 'Предыдущий месяц' });
  for (let i = 0; i < 36 && !(await page.getByText('Сентябрь 2026').first().isVisible()); i++) {
    await previous.click();
  }
  await page
    .getByRole('button', { name: /^Добавить трату в/ })
    .first()
    .click();
  await page.getByLabel(/^Наименование/).fill('Без анимации');
  await page.getByLabel('Сумма, ₽').fill('100');
  await page.keyboard.press('Enter');
  const row = page.getByRole('listitem').filter({ hasText: 'Без анимации' });
  await expect(row).toBeVisible();
  expect(await row.evaluate((el) => el.getAnimations().length)).toBe(0);
});

import { expect, test } from '@playwright/test';

test('график: canvas рисуется, подписи и таблица для скринридера на месте, консоль чистая', async ({
  page
}) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  page.on('pageerror', (e) => problems.push(e.message));

  await page.goto('/dev/components?vault=open');
  const plot = page.getByRole('img', { name: 'Статусы по месяцам' }).first();
  await plot.scrollIntoViewIfNeeded();
  await expect(plot.locator('canvas')).toBeVisible();

  const box = await plot.boundingBox();
  if (box === null) throw new Error('у графика нет размеров');
  expect(box.width).toBeGreaterThan(200);

  // Легенда и скрытая таблица дублируют данные не цветом.
  await expect(page.getByText('Оплачено').first()).toBeVisible();
  await expect(page.getByRole('table', { name: 'Статусы по месяцам' }).first()).toBeAttached();

  // Тултип открывается по наведению и содержит значение в ru-RU.
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.getByText(/\d\s₽/).first()).toBeVisible();

  expect(problems).toEqual([]);
});

test('график: смена темы перекрашивает canvas без перезагрузки', async ({ page }) => {
  await page.goto('/dev/components?vault=open');
  const plot = page.getByRole('img', { name: 'Статусы по месяцам' }).first();
  await plot.scrollIntoViewIfNeeded();
  const canvas = plot.locator('canvas');
  await expect(canvas).toBeVisible();
  const before = await canvas.screenshot();
  await page.evaluate(() => {
    document.documentElement.dataset.theme = 'dark';
  });
  await expect.poll(async () => (await canvas.screenshot()).equals(before)).toBe(false);
});

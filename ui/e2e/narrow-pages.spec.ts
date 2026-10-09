import { expect, test } from '@playwright/test';

const columns = [
  { route: '/settings', selector: '.card' },
  { route: '/help', selector: '.screen' }
];

test('Настройки и Справка имеют одинаковую ширину колонки', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const widths: number[] = [];
  for (const { route, selector } of columns) {
    await page.goto(`${route}?vault=open`);
    const column = page.locator(selector).first();
    await expect(column).toBeVisible();
    const box = await column.boundingBox();
    if (box === null) throw new Error(`нет геометрии колонки на ${route}`);
    widths.push(Math.round(box.width));
  }
  expect(new Set(widths).size, `ширины колонок: ${widths.join(', ')}`).toBe(1);
});

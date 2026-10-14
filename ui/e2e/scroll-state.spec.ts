import { expect, test } from '@playwright/test';

test('прокрутка: на корне ставится data-scrolling, графики не ловят указатель и потом снова ловят', async ({
  page
}) => {
  await page.setViewportSize({ width: 1280, height: 500 });
  await page.goto('/analytics?vault=open');
  const plot = page.locator('.plot').first();
  await expect(plot).toBeVisible();
  await expect(page.locator('html')).not.toHaveAttribute('data-scrolling', '1');

  await page.mouse.move(640, 250);
  await page.mouse.wheel(0, 200);
  await expect(page.locator('html')).toHaveAttribute('data-scrolling', '1');
  await expect(plot).toHaveCSS('pointer-events', 'none');

  await expect(page.locator('html')).not.toHaveAttribute('data-scrolling', '1');
  await expect(plot).toHaveCSS('pointer-events', 'auto');
});

test('графики после остановки прокрутки: подсказка по наведению и переход к записям по клику', async ({
  page
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('/analytics?vault=open');
  const plot = page.locator('.plot').first();
  await expect(plot.locator('canvas')).toBeVisible();

  await page.mouse.move(640, 300);
  await page.mouse.wheel(0, 150);
  await expect(page.locator('html')).toHaveAttribute('data-scrolling', '1');
  await page.mouse.wheel(0, -150);
  await expect(page.locator('html')).not.toHaveAttribute('data-scrolling', '1');
  await expect(plot).toHaveCSS('pointer-events', 'auto');

  const box = await plot.boundingBox();
  if (!box) throw new Error('нет раскладки графика');
  await page.mouse.move(box.x + box.width * 0.2, box.y + box.height * 0.5);
  await expect(plot).toContainText('Доходы');

  // Подсказка показывается над любой точкой оси, а клик ловит только сам элемент ряда, и его положение
  // зависит от раскладки и шрифтов: перебираем точки графика, пока клик не попадёт в элемент.
  await expect(async () => {
    for (const x of [0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.1, 0.9]) {
      for (const y of [0.5, 0.65, 0.75, 0.82, 0.35]) {
        await page.mouse.move(box.x + box.width * x, box.y + box.height * y);
        await page.mouse.click(box.x + box.width * x, box.y + box.height * y);
        if (page.url().includes('/expenses')) return;
      }
    }
    expect(page.url()).toContain('/expenses');
  }).toPass({ timeout: 20_000 });
});

test('быстрый уход с «Аналитики» и возврат: графики собраны, не пустые', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('/analytics?vault=open');
  await page.getByRole('link', { name: 'Расходы', exact: true }).click();
  await page.getByRole('link', { name: 'Аналитика', exact: true }).click();
  await page.getByRole('link', { name: 'Расходы', exact: true }).click();
  await page.getByRole('link', { name: 'Аналитика', exact: true }).click();
  const first = page.locator('.plot').first();
  await expect(first.locator('canvas')).toBeVisible();
  const size = await first
    .locator('canvas')
    .evaluate((c: HTMLCanvasElement) => [c.width, c.height]);
  expect(size[0]).toBeGreaterThan(100);
  expect(size[1]).toBeGreaterThan(100);
  await expect(page.locator('.plot canvas').nth(2)).toBeAttached();
});

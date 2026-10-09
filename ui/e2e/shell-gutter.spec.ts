import { expect, test } from '@playwright/test';

// Playwright по умолчанию прячет полосы прокрутки, и сдвиг не воспроизводится.
test.use({ launchOptions: { ignoreDefaultArgs: ['--hide-scrollbars'] } });

const routes = ['/', '/expenses', '/incomes', '/debts', '/savings', '/help'];

test('шапка не сдвигается между экранами с полосой прокрутки и без неё', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  const edges: number[] = [];
  for (const route of routes) {
    await page.goto(`${route}?vault=open`);
    const search = page.getByRole('button', { name: /Поиск и команды/ });
    await expect(search).toBeVisible();
    const box = await search.boundingBox();
    if (box === null) throw new Error(`нет геометрии поиска на ${route}`);
    edges.push(Math.round(box.x + box.width));
  }
  expect(new Set(edges).size, `правые края поиска: ${edges.join(', ')}`).toBe(1);
});

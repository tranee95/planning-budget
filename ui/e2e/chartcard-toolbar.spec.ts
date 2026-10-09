import { expect, test } from '@playwright/test';

// Ширины окна около 1280 px: карточка размера M на 4–8 px уже, чем нужно заголовку и панели.
for (const width of [1256, 1262, 1268, 1274, 1280]) {
  test(`карточка графика M при окне ${String(width)} px: панель в одной строке с заголовком`, async ({
    page
  }) => {
    await page.setViewportSize({ width, height: 800 });
    await page.goto('/analytics?vault=open');
    const card = page.getByRole('region', { name: 'Доходы, расходы и сбережения' });
    await expect(card).toBeVisible();
    const title = await card.getByRole('heading', { level: 2 }).boundingBox();
    const tools = await card.locator('.tools').boundingBox();
    if (title === null || tools === null) throw new Error('нет геометрии заголовка или панели');
    expect(tools.y, 'панель ушла под заголовок').toBeLessThan(title.y + title.height);
  });
}

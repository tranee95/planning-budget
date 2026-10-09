import { expect, test } from '@playwright/test';

test('карточка категории на Расходах: сверху полоса цвета категории', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/expenses?vault=open');
  const card = page.locator('article.card').first();
  await expect(card).toBeVisible();
  const { category, stripe } = await card.evaluate((el) => {
    const resolve = (color: string): string => {
      const probe = document.createElement('div');
      probe.style.color = color;
      document.body.append(probe);
      const rgb = getComputedStyle(probe).color;
      probe.remove();
      return rgb;
    };
    const own = getComputedStyle(el).getPropertyValue('--category-color').trim();
    return {
      category: own === '' ? '' : resolve(own),
      stripe: getComputedStyle(el, '::before').backgroundColor
    };
  });
  expect(category, 'у карточки нет --category-color').not.toBe('');
  expect(stripe).toBe(category);
});

import { expect, test } from '@playwright/test';
import { openSeptember } from './september';

test('«Расходы»: на карточках категорий нет значка «?», на «Итогах месяца» он есть', async ({
  page
}) => {
  await openSeptember(page);
  const cards = page.locator('article.card');
  await expect(cards.first()).toBeVisible();
  await expect(cards.getByRole('button', { name: /^Как считается/ })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Как считается: Итоги месяца' })).toBeVisible();
});

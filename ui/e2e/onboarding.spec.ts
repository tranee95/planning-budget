import { expect, test } from '@playwright/test';

test('подсказки: три шага, «Понятно» закрывает их, переключатель в настройках возвращает', async ({
  page
}) => {
  await page.goto('/?vault=open&tips=1');
  const tips = page.getByRole('region', { name: 'Подсказки для начала работы' });
  await expect(tips).toContainText('1 из 3');
  await tips.getByRole('button', { name: 'Далее' }).click();
  await expect(tips).toContainText('2 из 3');
  await tips.getByRole('button', { name: 'Далее' }).click();
  await expect(tips).toContainText('3 из 3');
  await tips.getByRole('button', { name: 'Понятно' }).click();
  await expect(tips).toHaveCount(0);

  await page.getByRole('link', { name: 'Настройки' }).click();
  const toggle = page.getByRole('switch', { name: 'Подсказки для начала работы' });
  await expect(toggle).not.toBeChecked();
  await page.getByText('Подсказки для начала работы', { exact: true }).click();
  await expect(page.getByRole('region', { name: 'Подсказки для начала работы' })).toBeVisible();
});

test('подсказки: «Больше не показывать» закрывает карточку на любом шаге', async ({ page }) => {
  await page.goto('/?vault=open&tips=1');
  const tips = page.getByRole('region', { name: 'Подсказки для начала работы' });
  await tips.getByRole('button', { name: 'Далее' }).click();
  await tips.getByRole('button', { name: 'Больше не показывать' }).click();
  await expect(tips).toHaveCount(0);
});

test('подсказки стоят над экраном и не закрывают его элементы на минимальном окне', async ({
  page
}) => {
  await page.setViewportSize({ width: 960, height: 600 });
  await page.goto('/?vault=open&tips=1');
  const tips = await page
    .getByRole('region', { name: 'Подсказки для начала работы' })
    .boundingBox();
  const kpi = await page.getByText('Доход', { exact: true }).first().boundingBox();
  expect(tips).not.toBeNull();
  expect(kpi).not.toBeNull();
  if (tips && kpi) {
    expect(tips.x + tips.width).toBeLessThanOrEqual(960);
    expect(tips.y + tips.height).toBeLessThanOrEqual(kpi.y);
  }
});

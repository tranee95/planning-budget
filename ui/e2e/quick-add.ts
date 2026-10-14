import { expect, type Page } from '@playwright/test';

/**
 * Окно быстрого добавления траты открывается из пустого состояния «Расходов».
 * В моках данные лежат в сентябре 2026; октябрь получает перенесённый план, и функция удаляет его траты.
 */
export async function openQuickAdd(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/expenses?vault=open');
  const previous = page.getByRole('button', { name: 'Предыдущий месяц' });
  for (let i = 0; i < 36 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await previous.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
  await page.getByRole('button', { name: 'Следующий месяц' }).click();
  await expect(page.getByText('Октябрь 2026')).toBeVisible();
  const remove = page.getByRole('button', { name: 'Удалить', exact: true });
  await expect(remove.first()).toBeVisible();
  while ((await remove.count()) > 0) {
    await remove.first().click();
  }
  await page.getByRole('button', { name: /Добавить трату/ }).click();
  await expect(page.getByRole('dialog', { name: 'Новая трата' })).toBeVisible();
}

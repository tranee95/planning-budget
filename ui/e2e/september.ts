import { expect, type Page } from '@playwright/test';

/**
 * Открывает «Расходы» на сентябре 2026: в моках траты лежат там. Текущий месяц при первом
 * открытии пуст (план переносится в него после загрузки экрана), поэтому вместо карточек
 * показывается пустое состояние (UX-7).
 */
export async function openSeptember(page: Page): Promise<void> {
  await page.goto('/expenses?vault=open');
  const previous = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 36 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await previous.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
}

import { expect, type Locator, type Page } from '@playwright/test';

/** Создаёт цель через форму «Новая цель» в шапке секции. */
export async function addGoal(page: Page, name: string, plan?: string): Promise<void> {
  await page.getByRole('button', { name: /Новая цель/ }).click();
  const form = page.getByRole('form', { name: 'Новая цель' });
  await form.getByLabel('Название цели').fill(name);
  if (plan !== undefined) await form.getByLabel('План на месяц, ₽').fill(plan);
  await form.getByRole('button', { name: 'Создать' }).click();
  await expect(form).toBeHidden();
}

/** Раскрывает карточку, если она свёрнута (при трёх и более целях раскрыта только первая). */
export async function expand(card: Locator): Promise<void> {
  const chevron = card.getByRole('button', { name: /(Развернуть|Свернуть) накопление/ });
  if ((await chevron.getAttribute('aria-expanded')) === 'false') await chevron.click();
}

/** Вносит сумму в строку ввода карточки; название необязательно. */
export async function addAmount(card: Locator, amount: string, title?: string): Promise<void> {
  await expand(card);
  if (title !== undefined) await card.getByLabel('Комментарий').fill(title);
  await card.getByLabel('Сумма, ₽').fill(amount);
  await card.getByLabel('Сумма, ₽').press('Enter');
}

/** Пункт меню «⋯» карточки. */
export async function pickMenu(card: Locator, item: string): Promise<void> {
  await card.getByRole('button', { name: /Действия с накоплением/ }).click();
  await card.getByRole('menuitem', { name: item }).click();
}

/** Строки-записи карточки (без строки ввода). */
export const records = (card: Locator): Locator => card.locator('.row.view');

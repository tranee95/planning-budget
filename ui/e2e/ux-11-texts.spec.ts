import { expect, test } from '@playwright/test';

// UX-11 (#188) и UX-10(б) (#187): мелкие тексты и подписи. По одному сценарию на экран.

test('долги: одна кнопка в пустом состоянии, подсказка под «Сохранить», тост и склонение', async ({
  page
}) => {
  await page.goto('/debts?vault=open');
  await expect(page.getByText('Долгов пока нет')).toBeVisible();
  await expect(page.getByRole('button', { name: '+ Долг' })).toHaveCount(1);
  await expect(page.getByRole('button', { name: 'Добавить долг' })).toHaveCount(0);

  await page.getByRole('button', { name: '+ Долг' }).click();
  const sheet = page.getByRole('dialog');
  const save = sheet.getByRole('button', { name: 'Сохранить' });
  await expect(save).toBeDisabled();
  await expect(sheet.getByText('Укажите, у кого или что')).toBeVisible();
  await sheet.getByLabel('У кого или что').fill('Брат');
  await expect(sheet.getByText('Введите сумму')).toBeVisible();
  await sheet.getByLabel('Сумма', { exact: true }).fill('30000');
  await expect(sheet.getByText('Введите сумму')).toHaveCount(0);
  await save.click();

  await expect(page.getByText('Долг «Брат» создан')).toBeVisible();
  await expect(page.getByText('1 открытый', { exact: true }).first()).toBeVisible();
});

test('категории: заголовки колонок, подсказка к балансу, пустое состояние с кнопкой', async ({
  page
}) => {
  await page.goto('/categories?vault=open');
  await expect(page.getByText('Среднее за год', { exact: true })).toBeVisible();
  await expect(page.getByText('Превышение лимита', { exact: true })).toBeVisible();
  await expect(page.getByText('Выше лимита', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Как считается: Баланс бюджета' })).toBeVisible();

  await page.goto('/categories?vault=open&fresh=1');
  await expect(page.getByText('Категорий пока нет')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Добавить категорию' })).toHaveCount(1);
  await page.getByRole('button', { name: 'Добавить категорию' }).click();
  await expect(page.getByRole('textbox', { name: 'Название' })).toBeVisible();
});

test('обзор: «Зафиксировать план» показывает тост с «Разблокировать»', async ({ page }) => {
  await page.goto('/?vault=open');
  // в пустом октябре вместо кнопки призыв составить план, поэтому берём сентябрь с данными
  await page.getByRole('button', { name: 'Предыдущий месяц' }).click();
  await page.getByRole('button', { name: 'Зафиксировать план' }).click();
  await expect(page.getByText(/План на .+ \d{4} зафиксирован/)).toBeVisible();
  const toast = page.getByRole('button', { name: 'Разблокировать' });
  await expect(toast).toHaveCount(2);
  await toast.last().click();
  await expect(page.getByRole('button', { name: 'Зафиксировать план' })).toBeVisible();
});

test('справка: порядок блоков, шаг знакомства короче, ключ и сеть описаны верно', async ({
  page
}) => {
  await page.goto('/help?vault=open');
  await expect(page.getByRole('heading', { name: 'Цикл месяца' })).toBeVisible();
  const headings = await page.getByRole('heading').allTextContents();
  const index = (title: string): number => headings.findIndex((h) => h.includes(title));
  expect(index('Цикл месяца')).toBeGreaterThanOrEqual(0);
  expect(index('Как спланировать месяц')).toBe(index('Цикл месяца') + 1);
  expect(index('Как спланировать месяц')).toBeLessThan(index('Данные остаются у вас'));
  await expect(page.getByText('кроме проверки обновлений')).toBeVisible();
  await expect(page.getByText('«Настройки → Вид»')).toBeVisible();

  await page.goto('/?vault=open&intro=1');
  for (let i = 0; i < 3; i++) await page.getByRole('button', { name: 'Далее' }).click();
  const dialog = page.getByRole('dialog', { name: 'Сбережения и долги' });
  await expect(dialog).toContainText('Шаг 4 из 5');
  await expect(dialog).not.toContainText('Закрыть цель');
  await page.getByRole('button', { name: 'Далее' }).click();
  const last = page.getByRole('dialog', { name: 'Данные остаются у вас' });
  await expect(last).toContainText('Ключ восстановления вы получили при создании хранилища');
  await expect(last).toContainText('кроме проверки обновлений');
});

test('настройки: перевыпуск ключа вынесен в «Опасные действия» внизу', async ({ page }) => {
  await page.goto('/settings?vault=open');
  const danger = page.getByRole('region', { name: 'Опасные действия' });
  await expect(danger.getByRole('heading', { name: 'Перевыпуск ключа шифрования' })).toBeVisible();
  await expect(danger.getByText('прежний ключ восстановления перестанет работать')).toBeVisible();
  const security = page.getByRole('region', { name: 'Безопасность' });
  await expect(security.getByText('Перевыпуск ключа шифрования')).toHaveCount(0);
  const dangerBox = await danger.boundingBox();
  const securityBox = await security.boundingBox();
  expect(dangerBox?.y ?? 0).toBeGreaterThan(securityBox?.y ?? Infinity);
});

test('обзор: пустой месяц — призыв составить план вместо нулей, с «Скопировать план из …»', async ({
  page
}) => {
  await page.goto('/?vault=open');
  const next = page.getByRole('button', { name: 'Следующий месяц' });
  const heading = page.getByRole('heading', { name: /Составьте план на/ });
  for (let i = 0; i < 6 && !(await heading.isVisible()); i++) await next.click();
  await expect(heading).toBeVisible();
  await expect(page.getByRole('button', { name: 'Начать с нуля' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Зафиксировать план' })).toHaveCount(0);
});

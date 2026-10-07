import { expect, test } from '@playwright/test';

test('знакомство: пять шагов, «Позже» закрывает окно', async ({ page }) => {
  await page.goto('/?vault=open&intro=1');
  const dialog = page.getByRole('dialog', { name: 'Месяц начинается с плана' });
  await expect(dialog).toContainText('Шаг 1 из 5');
  await dialog.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: 'Цикл месяца' })).toContainText('Шаг 2 из 5');
  await page.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: 'Статусы трат' })).toContainText('Незапланировано');
  await page.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: 'Данные остаются у вас' })).toContainText(
    'Шаг 5 из 5'
  );
  await page.getByRole('button', { name: 'Позже' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('знакомство: стрелки листают шаги, «Назад» возвращает', async ({ page }) => {
  await page.goto('/?vault=open&intro=1');
  await expect(page.getByRole('dialog', { name: 'Месяц начинается с плана' })).toBeVisible();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('dialog', { name: 'Цикл месяца' })).toBeVisible();
  await page.getByRole('button', { name: 'Назад' }).click();
  await expect(page.getByRole('dialog', { name: 'Месяц начинается с плана' })).toBeVisible();
});

test('мастер первого месяца: доход, статьи, накопления, итог и «План готов»', async ({ page }) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  await page.goto('/?vault=open&intro=1');
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('button', { name: 'Спланировать первый месяц' }).click();

  const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
  await expect(wizard).toBeVisible();
  await wizard.getByLabel('Сумма').fill('100000');
  await wizard.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: /: Статьи/ })).toBeVisible();
  await page.getByRole('dialog').getByLabel('Продукты', { exact: true }).fill('30000');
  await page.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: /: Сбережения/ })).toContainText('Накопления');
  await page.getByRole('button', { name: 'Далее' }).click();

  const result = page.getByRole('dialog', { name: /: Итог/ });
  await expect(result).toContainText('Не распределено');
  await result.getByRole('button', { name: 'План готов' }).click();
  await expect(page.getByText(/План на .+ готов/)).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(problems).toEqual([]);
});

test('настройки: «Показать знакомство снова» открывает знакомство', async ({ page }) => {
  await page.goto('/settings?vault=open');
  await page.getByRole('button', { name: 'Показать знакомство снова' }).click();
  await expect(page.getByRole('dialog', { name: 'Месяц начинается с плана' })).toBeVisible();
});

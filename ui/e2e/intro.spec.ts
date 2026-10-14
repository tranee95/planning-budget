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
  // Первый месяц: пустое хранилище (мастер отказывает, если в месяце уже есть плановые строки).
  await page.goto('/?vault=open&intro=1&fresh=1');
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();
  await page
    .getByRole('dialog', { name: 'Категории расходов' })
    .getByRole('button', { name: 'Далее' })
    .click();
  await page.getByRole('button', { name: 'Спланировать первый месяц' }).click();

  const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
  await expect(wizard).toBeVisible();
  await wizard.getByLabel('Сумма').fill('100000');
  await wizard.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: /: Статьи/ })).toBeVisible();
  await page.getByRole('dialog').getByLabel('Продукты', { exact: true }).fill('30000');
  await page.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: /: Сбережения/ })).toContainText('Сбережения');
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

test('знакомство: выбор категорий при первом запуске, обязательные закреплены', async ({
  page
}) => {
  await page.goto('/?vault=open&intro=1&fresh=1');
  const dialog = page.getByRole('dialog', { name: 'Месяц начинается с плана' });
  await expect(dialog).toContainText('Шаг 1 из 6');
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();

  const picker = page.getByRole('dialog', { name: 'Категории расходов' });
  await expect(picker).toContainText('Шаг 5 из 6');
  await expect(picker.getByRole('checkbox', { name: /Сбережения/ })).toBeDisabled();
  await expect(picker.getByRole('checkbox', { name: /Прочее/ })).toBeChecked();
  await picker.getByRole('button', { name: 'Только обязательные' }).click();
  await expect(picker.getByRole('checkbox', { name: 'Продукты' })).not.toBeChecked();
  await picker.getByRole('checkbox', { name: 'Подписки' }).check();

  await picker.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('button', { name: 'Позже' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);

  await page.getByRole('link', { name: 'Категории', exact: true }).click();
  const list = page.getByRole('main');
  await expect(list).toContainText('Подписки');
  await expect(list).toContainText('Прочее');
  await expect(list).not.toContainText('Продукты');
});

test('новый сейф: после выбора категорий мастер показывает статьи и накопления', async ({
  page
}) => {
  await page.goto('/?vault=open&intro=1&fresh=1');
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();
  const picker = page.getByRole('dialog', { name: 'Категории расходов' });
  await picker.getByRole('button', { name: 'Только обязательные' }).click();
  await picker.getByRole('checkbox', { name: 'Подписки' }).check();
  await picker.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('button', { name: 'Спланировать первый месяц' }).click();

  const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
  await wizard.getByLabel('Сумма').fill('100000');
  await wizard.getByRole('button', { name: 'Далее' }).click();
  const articles = page.getByRole('dialog', { name: /: Статьи/ });
  await expect(articles.getByLabel('Подписки', { exact: true })).toBeVisible();
  await expect(articles.getByLabel('Прочее', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Далее' }).click();
  await expect(page.getByRole('dialog', { name: /: Сбережения/ })).toContainText('Сбережения');
});

test('мастер в месяце, где план уже есть, сразу объясняет и не пускает дальше', async ({
  page
}) => {
  // Обзор текущего месяца переносит план прошлого месяца (U-1), после этого мастер не нужен.
  await page.goto('/?vault=open');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await page.getByRole('link', { name: 'Справка' }).click();
  await page.getByRole('button', { name: 'Спланировать месяц' }).click();
  const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
  await expect(wizard).toContainText('уже есть план');
  await expect(wizard.getByRole('button', { name: 'Далее' })).toBeDisabled();
});

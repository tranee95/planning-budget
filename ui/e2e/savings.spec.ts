import { expect, test } from '@playwright/test';
import { openSeptember } from './september';

test('сбережения: итог месяца, карточка накопления с суммами, без процентов и ставок', async ({
  page
}) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  await page.goto('/savings?vault=open');
  await expect(page.getByRole('heading', { level: 1, name: 'Сбережения' })).toBeVisible();
  await expect(page.getByRole('heading', { level: 2, name: 'Накопления' })).toBeVisible();
  await expect(page.getByRole('button', { name: /Добавить сумму/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /Предыдущий месяц/ })).toBeVisible();

  const body = page.getByRole('main');
  await expect(body).toContainText('Накоплено всего');
  await expect(body).toContainText('Отложено по месяцам');
  for (const gone of [
    'К концу года',
    'Прогноз',
    'Ставка',
    'Налог',
    '% от дохода',
    'Доля дохода',
    'Баланс накоплений',
    'Брокерск',
    'Отложить'
  ]) {
    await expect(body).not.toContainText(gone);
  }
  expect(problems).toEqual([]);
});

test('сбережения: 30 000 ₽ тремя суммами по 10 000 ₽ — три строки одной карточки', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  for (let i = 0; i < 3; i++) {
    await card.getByRole('button', { name: /Добавить сумму/ }).click();
    await page.getByLabel('Сумма, ₽').fill('10000');
    await page.keyboard.press('Enter');
    await expect(card.getByRole('listitem').filter({ hasText: /10\s000/ })).toHaveCount(i + 1);
  }
  // Не три карточки, а одна, с итогом 30 000 ₽.
  await expect(page.getByRole('article')).toHaveCount(1);
  await expect(card.locator('header')).toContainText(/30\s000/);
});

test('сбережения: сумму можно изменить, удалить и вернуть', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: /Добавить сумму/ }).click();
  await page.getByLabel('Наименование, Накопления').fill('Аванс');
  await page.getByLabel('Сумма, ₽').fill('5000');
  await page.keyboard.press('Enter');
  const row = card.getByRole('listitem').filter({ hasText: 'Аванс' });
  await expect(row).toContainText(/5\s000/);

  await row.getByRole('button', { name: /Аванс/ }).click();
  await page.getByLabel('Сумма, ₽').fill('7000');
  await page.keyboard.press('Enter');
  await expect(row).toContainText(/7\s000/);
  await expect(card.locator('header')).toContainText(/7\s000/);

  await row.getByRole('button', { name: 'Удалить' }).click();
  await expect(row).toBeHidden();
  await expect(card.locator('header')).toContainText(/0\s₽/);
  await page.getByRole('button', { name: 'Вернуть' }).click();
  await expect(row).toBeVisible();
  await expect(card.locator('header')).toContainText(/7\s000/);
});

test('сбережения: у каждого месяца свои суммы, новый месяц пуст', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: /Добавить сумму/ }).click();
  await page.getByLabel('Сумма, ₽').fill('4000');
  await page.keyboard.press('Enter');
  await expect(card.getByRole('listitem').filter({ hasText: /4\s000/ })).toHaveCount(1);

  await page.getByRole('button', { name: /Предыдущий месяц/ }).click();
  await expect(card.getByText('ещё ничего не откладывали')).toBeVisible();
  await page.getByRole('button', { name: /Следующий месяц/ }).click();
  await expect(card.getByRole('listitem').filter({ hasText: /4\s000/ })).toHaveCount(1);
});

test('сбережения: план на месяц сохраняется кнопкой', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  const save = card.getByRole('button', { name: 'Сохранить план' });
  await expect(save).toBeDisabled();
  await card.getByLabel('План на месяц').fill('20000');
  await expect(save).toBeEnabled();
  await save.click();
  await expect(save).toBeDisabled();
  await expect(page.locator('.kpis')).toContainText(/20\s000/);
});

test('сбережения не появляются карточкой на «Расходах» и «Категориях»', async ({ page }) => {
  await openSeptember(page);
  await expect(page.getByRole('heading', { name: 'Продукты' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Накопления' })).toHaveCount(0);
  await page.goto('/categories?vault=open');
  await expect(page.getByRole('heading', { name: 'Категории' })).toBeVisible();
  await expect(page.getByText('Накопления', { exact: true })).toHaveCount(0);
});

test('сбережения: клавиша 6 ведёт на раздел, а /bonds перенаправляет на него', async ({ page }) => {
  await page.goto('/?vault=open');
  await page.locator('body').click({ position: { x: 700, y: 400 } });
  await page.keyboard.press('Digit6');
  await expect(page).toHaveURL(/\/savings$/);

  await page.goto('/bonds?vault=open');
  await expect(page).toHaveURL(/\/savings$/);
});

for (const [width, theme] of [
  [960, 'light'],
  [1280, 'dark']
] as const) {
  test(`сбережения: карточка не вылезает за окно и читается (${String(width)} px, ${theme})`, async ({
    page
  }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.setViewportSize({ width, height: 700 });
    await page.goto('/savings?vault=open');
    const card = page.getByRole('article', { name: 'Накопления' });
    await card.getByRole('button', { name: /Добавить сумму/ }).click();
    await page
      .getByLabel('Наименование, Накопления')
      .fill('Очень длинное название суммы '.repeat(4));
    await page.getByLabel('Сумма, ₽').fill('999999');
    await page.keyboard.press('Enter');
    await expect(card.getByRole('listitem').filter({ hasText: /999\s999/ })).toHaveCount(1);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    );
    expect(overflow).toBeLessThanOrEqual(0);
    const box = await card.boundingBox();
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(width);
    await expect(card.getByRole('button', { name: 'Сохранить план' })).toBeVisible();
    await expect(card.getByRole('button', { name: 'Удалить' })).toBeVisible();
  });
}

test('сбережения: цель закрывается с подтверждением и возвращается из закрытых', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: /Добавить сумму/ }).click();
  await page.getByLabel('Сумма, ₽').fill('5000');
  await page.keyboard.press('Enter');
  await expect(card.locator('header')).toContainText(/5\s000/);

  const totalBefore = await page.locator('.kpis').innerText();

  await card.getByRole('button', { name: 'Закрыть цель Накопления' }).click();
  const dialog = page.getByRole('dialog', { name: 'Закрыть цель?' });
  await expect(dialog).toContainText('Накопления');
  await dialog.getByRole('button', { name: 'Отмена' }).click();
  await expect(dialog).toBeHidden();
  await expect(card).toBeVisible();

  await card.getByRole('button', { name: 'Закрыть цель Накопления' }).click();
  await dialog.getByRole('button', { name: 'Закрыть цель' }).click();
  await expect(page.getByRole('article', { name: 'Накопления' })).toHaveCount(0);

  // Накопленное закрытой цели остаётся в итоге.
  expect(await page.locator('.kpis').innerText()).toBe(totalBefore);
  await page.getByText(/Показать закрытые цели/).click();
  const goal = page.getByRole('listitem').filter({ hasText: 'Накопления' });
  await expect(goal).toContainText(/накоплено\s+\d+\s000\s₽/);
  await goal.getByRole('button', { name: 'Вернуть цель Накопления' }).click();
  await expect(page.getByRole('article', { name: 'Накопления' })).toBeVisible();
});

test('сбережения: кнопка добавления накопления обычного размера, по высоте как поле', async ({
  page
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/savings?vault=open');
  const input = page.getByLabel('Новое накопление');
  const button = page.getByRole('button', { name: 'Добавить накопление' });
  const [i, b] = [await input.boundingBox(), await button.boundingBox()];
  if (!i || !b) throw new Error('нет раскладки');
  expect(Math.abs(i.height - b.height)).toBeLessThanOrEqual(1);
  expect(b.width).toBeLessThan(260);
  // Кнопка стоит справа от поля, а не под ним и не на всю ширину.
  expect(b.x).toBeGreaterThan(i.x + i.width - 1);
  expect(Math.abs(b.y + b.height / 2 - (i.y + i.height / 2))).toBeLessThanOrEqual(2);
});

test('сбережения: график показывает все накопления, в том числе закрытую цель', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  await page.getByLabel('Новое накопление').fill('Отпуск');
  await page.getByRole('button', { name: 'Добавить накопление' }).click();
  for (const name of ['Накопления', 'Отпуск']) {
    const card = page.getByRole('article', { name });
    await card.getByRole('button', { name: /Добавить сумму/ }).click();
    await page.getByLabel('Сумма, ₽').fill('3000');
    await page.keyboard.press('Enter');
    await expect(card.locator('header')).toContainText(/3\s000/);
  }
  const legend = page.locator('figure .legend');
  await expect(legend).toContainText('Накопления');
  await expect(legend).toContainText('Отпуск');

  await page
    .getByRole('article', { name: 'Отпуск' })
    .getByRole('button', { name: /Закрыть цель/ })
    .click();
  await page
    .getByRole('dialog', { name: 'Закрыть цель?' })
    .getByRole('button', { name: 'Закрыть цель' })
    .click();
  await expect(page.getByRole('article', { name: 'Отпуск' })).toHaveCount(0);
  // Закрытая цель остаётся на графике.
  await expect(legend).toContainText('Отпуск');
});

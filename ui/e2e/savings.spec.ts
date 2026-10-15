import { expect, test } from '@playwright/test';
import { addAmount, addGoal, expand, pickMenu, records } from './savings-helpers';
import { openSeptember } from './september';

test('сбережения: итоги, график, карточка накопления, без процентов и ставок', async ({ page }) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  await page.goto('/savings?vault=open');
  await expect(page.getByRole('heading', { level: 1, name: 'Сбережения' })).toBeVisible();
  await expect(page.getByRole('heading', { level: 2, name: 'Накопления' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Добавить сумму' })).toBeVisible();
  await expect(page.getByRole('button', { name: /Новая цель/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /Предыдущий месяц/ })).toBeVisible();

  const body = page.getByRole('main');
  await expect(body).toContainText('Накоплено всего');
  await expect(body).toContainText('Отложено по месяцам');
  await expect(body).toContainText('Всего отложено в');
  for (const gone of [
    'К концу года',
    'Прогноз',
    'Ставка',
    'Налог',
    '% от дохода',
    'Доля дохода',
    'Баланс накоплений',
    'Брокерск',
    'Сохранить план'
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
    await addAmount(card, '10000');
    await expect(records(card).filter({ hasText: /10\s000/ })).toHaveCount(i + 1);
  }
  await expect(page.getByRole('article')).toHaveCount(1);
  await expect(card.locator('header')).toContainText(/30\s000/);
});

test('сбережения: сумму можно изменить, удалить и вернуть', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await addAmount(card, '5000', 'Аванс');
  const row = records(card).filter({ hasText: 'Аванс' });
  await expect(row).toContainText(/5\s000/);

  await row.getByRole('button', { name: /Изменить сумму/ }).click();
  await card.getByLabel('Сумма, ₽').first().fill('7000');
  await card.getByLabel('Сумма, ₽').first().press('Enter');
  await expect(row).toContainText(/7\s000/);
  await expect(card.locator('header')).toContainText(/7\s000/);

  await row.getByRole('button', { name: /Удалить запись/ }).click();
  await expect(row).toBeHidden();
  await expect(card.locator('header')).toContainText(/0\s₽/);
  await page.getByRole('button', { name: 'Вернуть', exact: true }).click();
  await expect(row).toBeVisible();
  await expect(card.locator('header')).toContainText(/7\s000/);
});

test('сбережения: пустая или нулевая сумма не принимается, ошибка у поля', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: 'Добавить сумму' }).click();
  await expect(card.getByRole('alert')).toContainText('Введите сумму больше нуля');
  await expect(card.getByLabel('Сумма, ₽')).toHaveAttribute('aria-invalid', 'true');
  await expect(records(card)).toHaveCount(0);
  await expect(card.getByText('Пока ничего не отложено')).toBeVisible();
});

test('сбережения: у каждого месяца свои суммы, новый месяц пуст', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await addAmount(card, '4000');
  await expect(records(card).filter({ hasText: /4\s000/ })).toHaveCount(1);

  await page.getByRole('button', { name: /Предыдущий месяц/ }).click();
  await expect(card.getByText('Пока ничего не отложено')).toBeVisible();
  await page.getByRole('button', { name: /Следующий месяц/ }).click();
  await expect(records(card).filter({ hasText: /4\s000/ })).toHaveCount(1);
});

test('сбережения: план меняется кликом по сумме в шапке, Enter сохраняет, Esc отменяет', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await expect(card.getByRole('button', { name: 'Сохранить план' })).toHaveCount(0);

  await card.getByRole('button', { name: /Изменить план на месяц/ }).click();
  const field = card.getByLabel('План на месяц, ₽');
  await expect(field).toBeFocused();
  await field.fill('20000');
  await field.press('Enter');
  await expect(card.locator('header')).toContainText(/20\s000/);
  await expect(page.locator('.kpis')).toContainText(/20\s000/);

  await card.getByRole('button', { name: /Изменить план на месяц/ }).click();
  await card.getByLabel('План на месяц, ₽').fill('99000');
  await card.getByLabel('План на месяц, ₽').press('Escape');
  await expect(card.locator('header')).toContainText(/20\s000/);
  await expect(card.locator('header')).not.toContainText(/99\s000/);
});

test('сбережения: прогресс плана и состояние «план выполнен»', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await card.getByRole('button', { name: /Изменить план на месяц/ }).click();
  await card.getByLabel('План на месяц, ₽').fill('10000');
  await card.getByLabel('План на месяц, ₽').press('Enter');

  await addAmount(card, '2500');
  const bar = card.getByRole('progressbar');
  await expect(bar).toHaveAttribute('aria-valuenow', '25');
  await expect(card.locator('header')).toContainText('25%');
  await expect(card.getByText('План выполнен')).toHaveCount(0);

  await addAmount(card, '7500');
  await expect(bar).toHaveAttribute('aria-valuenow', '100');
  await expect(card.getByText('План выполнен')).toBeVisible();
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
    await addAmount(card, '999999', 'Очень длинное название суммы '.repeat(4));
    await expect(records(card).filter({ hasText: /999\s999/ })).toHaveCount(1);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    );
    expect(overflow).toBeLessThanOrEqual(0);
    const box = await card.boundingBox();
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(width);
    await expect(card.getByRole('button', { name: /Удалить запись/ })).toBeVisible();
    await expect(card.getByRole('button', { name: 'Добавить сумму' })).toBeVisible();
  });
}

test('сбережения: суммы, поле суммы и колонка действий выровнены по одной сетке', async ({
  page
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await addAmount(card, '12345', 'Зарплата');
  const row = records(card).first();
  const [sum, remove, field, add] = await Promise.all([
    row.getByRole('button', { name: /Изменить сумму/ }).boundingBox(),
    row.getByRole('button', { name: /Удалить запись/ }).boundingBox(),
    card.getByLabel('Сумма, ₽').locator('..').boundingBox(),
    card.getByRole('button', { name: 'Добавить сумму' }).boundingBox()
  ]);
  if (!sum || !remove || !field || !add) throw new Error('нет раскладки');
  // Правый край суммы записи — край колонки, а поле суммы в строке ввода занимает ту же колонку.
  expect(Math.abs(sum.x + sum.width - (field.x + field.width))).toBeLessThanOrEqual(1);
  expect(Math.abs(remove.x + remove.width / 2 - (add.x + add.width / 2))).toBeLessThanOrEqual(1);
  // Все контролы строки ввода одной высоты.
  expect(Math.abs(field.height - add.height)).toBeLessThanOrEqual(1);
  expect(remove.height).toBeGreaterThanOrEqual(32);
});

test('сбережения: цель закрывается через меню с подтверждением и возвращается из закрытых', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  await addAmount(card, '5000');
  await expect(card.locator('header')).toContainText(/5\s000/);
  await expect(card.getByRole('button', { name: /^Закрыть цель/ })).toHaveCount(0);

  const totalBefore = await page.locator('.kpis').innerText();

  await pickMenu(card, 'Закрыть цель');
  const dialog = page.getByRole('dialog', { name: 'Закрыть цель?' });
  await expect(dialog).toContainText('Накопления');
  await dialog.getByRole('button', { name: 'Отмена' }).click();
  await expect(dialog).toBeHidden();
  await expect(card).toBeVisible();

  await pickMenu(card, 'Закрыть цель');
  await dialog.getByRole('button', { name: 'Закрыть цель' }).click();
  await expect(page.getByRole('article', { name: 'Накопления' })).toHaveCount(0);

  // Накопленное закрытой цели остаётся в итоге.
  expect(await page.locator('.kpis').innerText()).toBe(totalBefore);
  await page.getByRole('button', { name: /Закрытые цели/ }).click();
  const goal = page.getByRole('listitem').filter({ hasText: 'Накопления' });
  await expect(goal).toContainText(/накоплено\s+\d+\s000\s₽/);
  await goal.getByRole('button', { name: 'Вернуть цель Накопления' }).click();
  await expect(page.getByRole('article', { name: 'Накопления' })).toBeVisible();
});

test('сбережения: цель удаляется через меню с подтверждением, «Вернуть» возвращает её', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  await addGoal(page, 'Отпуск');
  const card = page.getByRole('article', { name: 'Отпуск' });
  await addAmount(card, '3000');
  await expect(card.locator('header')).toContainText(/3\s000/);

  await pickMenu(card, 'Удалить цель');
  const dialog = page.getByRole('dialog', { name: 'Удалить цель?' });
  await expect(dialog).toContainText(/3\s000/);
  await dialog.getByRole('button', { name: 'Удалить цель' }).click();
  await expect(card).toHaveCount(0);

  await page.getByRole('button', { name: 'Вернуть', exact: true }).click();
  await expect(card).toBeVisible();
  await expect(card.locator('header')).toContainText(/3\s000/);
});

test('сбережения: меню «⋯» закрывается по Esc и возвращает фокус', async ({ page }) => {
  await page.goto('/savings?vault=open');
  const card = page.getByRole('article', { name: 'Накопления' });
  const trigger = card.getByRole('button', { name: /Действия с накоплением/ });
  await trigger.click();
  await expect(card.getByRole('menuitem')).toHaveCount(3);
  await page.keyboard.press('Escape');
  await expect(card.getByRole('menu')).toHaveCount(0);
  await expect(trigger).toBeFocused();
});

test('сбережения: новая цель создаётся в шапке секции вместе с планом', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/savings?vault=open');
  await expect(page.getByLabel('Название цели')).toHaveCount(0);
  await addGoal(page, 'Отпуск', '30000');
  const card = page.getByRole('article', { name: 'Отпуск' });
  await expect(card.locator('header')).toContainText(/30\s000/);
  await expect(page.locator('.kpis')).toContainText(/30\s000/);
  // Кнопка создания стоит выше списка карточек.
  const [button, list] = [
    await page.getByRole('button', { name: /Новая цель/ }).boundingBox(),
    await page.getByRole('article').first().boundingBox()
  ];
  expect((button?.y ?? 0) < (list?.y ?? 0)).toBe(true);
});

test('сбережения: при трёх целях раскрыта только первая, остальные раскрываются по шеврону', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  await addGoal(page, 'Отпуск');
  await addGoal(page, 'Ремонт');
  const [first, second] = [
    page.getByRole('article', { name: 'Накопления' }),
    page.getByRole('article', { name: 'Отпуск' })
  ];
  await expect(first.getByRole('button', { name: /Свернуть накопление/ })).toHaveAttribute(
    'aria-expanded',
    'true'
  );
  await expect(second.getByRole('button', { name: /Развернуть накопление/ })).toHaveAttribute(
    'aria-expanded',
    'false'
  );
  await expect(second.getByLabel('Сумма, ₽')).toHaveCount(0);
  await expand(second);
  await expect(second.getByLabel('Сумма, ₽')).toBeVisible();
});

test('сбережения: график показывает все накопления, в том числе закрытую цель', async ({
  page
}) => {
  await page.goto('/savings?vault=open');
  await addGoal(page, 'Отпуск');
  for (const name of ['Накопления', 'Отпуск']) {
    const card = page.getByRole('article', { name });
    await addAmount(card, '3000');
    await expect(card.locator('header')).toContainText(/3\s000/);
  }
  const legend = page.locator('figure .legend');
  await expect(legend).toContainText('Накопления');
  await expect(legend).toContainText('Отпуск');

  await pickMenu(page.getByRole('article', { name: 'Отпуск' }), 'Закрыть цель');
  await page
    .getByRole('dialog', { name: 'Закрыть цель?' })
    .getByRole('button', { name: 'Закрыть цель' })
    .click();
  await expect(page.getByRole('article', { name: 'Отпуск' })).toHaveCount(0);
  // Закрытая цель остаётся на графике.
  await expect(legend).toContainText('Отпуск');
});

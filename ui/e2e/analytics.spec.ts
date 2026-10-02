import { expect, test, type Page } from '@playwright/test';

async function open(page: Page): Promise<void> {
  await page.goto('/analytics?vault=open');
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(9);
}

const titles = (page: Page): Promise<string[]> =>
  page.getByRole('heading', { level: 2 }).allTextContents();

test('аналитика: стандартный дашборд, девять карточек в порядке чтения', async ({ page }) => {
  await open(page);
  expect((await titles(page)).slice(0, 2)).toEqual([
    'Доходы, расходы и сбережения',
    'Накопления: остаток и сбережения'
  ]);
  await expect(page.getByRole('tab', { name: 'Мой бюджет' })).toHaveAttribute(
    'aria-selected',
    'true'
  );
});

test('перетаскивание за ручку меняет порядок и сохраняется после перезагрузки данных', async ({
  page
}) => {
  await open(page);
  const grip = page.getByRole('button', { name: /Переместить «Траты по статусам»/ });
  const first = page.getByRole('region', { name: 'Доходы, расходы и сбережения' });
  const from = await grip.boundingBox();
  const to = await first.boundingBox();
  if (from === null || to === null) throw new Error('нет геометрии');

  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + 20, to.y + 20, { steps: 12 });
  await page.mouse.up();

  await expect.poll(async () => (await titles(page))[0]).toBe('Траты по статусам');
});

test('клавиатура: стрелки на ручке сдвигают карточку', async ({ page }) => {
  await open(page);
  const card = page.getByRole('region', { name: 'Накопления: остаток и сбережения' });
  const before = (await card.boundingBox())?.x ?? 0;
  const grip = page.getByRole('button', { name: /Переместить «Накопления: остаток и сбережения»/ });
  await grip.focus();
  await page.keyboard.press('ArrowLeft');
  await expect.poll(async () => (await titles(page))[0]).toBe('Накопления: остаток и сбережения');
  // один шаг сетки: ширина колонки с промежутком, примерно 97 px при окне 1440
  await expect.poll(async () => before - ((await card.boundingBox())?.x ?? 0)).toBeGreaterThan(60);
});

test('размер S/M/L меняет ширину карточки', async ({ page }) => {
  await open(page);
  const card = page.getByRole('region', { name: 'Структура по типам' });
  const before = (await card.boundingBox())?.width ?? 0;
  await page
    .getByRole('group', { name: 'Размер «Структура по типам»' })
    .getByRole('button', { name: 'L' })
    .click();
  await expect
    .poll(async () => (await card.boundingBox())?.width ?? 0)
    .toBeGreaterThan(before * 1.5);
  await expect(
    page
      .getByRole('group', { name: 'Размер «Структура по типам»' })
      .getByRole('button', { name: 'L' })
  ).toHaveAttribute('aria-pressed', 'true');
});

test('удаление карточки отменяется тостом, карточка возвращается', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Удалить «Норма сбережений»' }).click();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(8);
  await page.getByRole('button', { name: 'Отменить' }).click();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(9);
  await expect(page.getByRole('region', { name: 'Норма сбережений' })).toBeVisible();
});

test('дублирование добавляет копию', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Дублировать «Структура по типам»' }).click();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(10);
  await expect(page.getByRole('region', { name: 'Структура по типам (копия)' })).toBeVisible();
});

test('несколько дашбордов: создание, переключение, переименование, удаление', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Новый дашборд' }).click();
  await page.getByLabel('Название').fill('Отпуск');
  await page.getByRole('button', { name: 'Создать' }).click();
  await expect(page.getByRole('tab', { name: 'Отпуск' })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByText('На дашборде пока нет графиков')).toBeVisible();

  await page.getByRole('button', { name: 'Переименовать дашборд' }).click();
  await page.getByLabel('Название').fill('Поездки');
  await page.getByRole('button', { name: 'Сохранить' }).click();
  await expect(page.getByRole('tab', { name: 'Поездки' })).toBeVisible();

  await page.getByRole('tab', { name: 'Мой бюджет' }).click();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(9);

  await page.getByRole('tab', { name: 'Поездки' }).click();
  await page.getByRole('button', { name: 'Удалить дашборд' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Удалить' }).click();
  await expect(page.getByRole('tab', { name: 'Поездки' })).toHaveCount(0);
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(9);
});

test('минимальное окно: страница без горизонтальной прокрутки, карточки в пределах сетки', async ({
  page
}) => {
  await page.setViewportSize({ width: 960, height: 600 });
  await open(page);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );
  expect(overflow).toBeLessThanOrEqual(0);
  const card = await page.getByRole('region', { name: 'Структура по типам' }).boundingBox();
  const viewport = page.viewportSize();
  expect((card?.x ?? 0) + (card?.width ?? 0)).toBeLessThanOrEqual(viewport?.width ?? 0);
});

test('конструктор: новый график с живым превью добавляется на панель', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'График', exact: true }).click();
  const panel = page.getByRole('dialog', { name: 'Новый график' });
  await expect(panel).toBeVisible();
  await expect(panel.getByRole('img', { name: 'Новый график' })).toBeVisible();

  // кольцо недоступно при разбивке по месяцам, причина — в подсказке
  const donut = panel.getByRole('radio', { name: 'Кольцо' });
  await expect(donut).toBeDisabled();
  await expect(donut).toHaveAttribute('title', /не по месяцам/);

  await panel.getByLabel('Название').fill('Траты по типам');
  await panel.getByLabel('Разбить по').selectOption('kind');
  await expect(donut).toBeEnabled();
  await donut.click();
  await expect(donut).toHaveAttribute('aria-checked', 'true');

  await panel.getByRole('button', { name: 'Добавить на панель' }).click();
  await expect(panel).toBeHidden();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(10);
  await expect(page.getByRole('region', { name: 'Траты по типам' })).toBeVisible();
});

test('конструктор: недопустимое сочетание показывает причину и закрывает сохранение', async ({
  page
}) => {
  await open(page);
  await page.getByRole('button', { name: 'График', exact: true }).click();
  const panel = page.getByRole('dialog', { name: 'Новый график' });
  await panel.getByLabel('Фильтр').fill('источник:импорт');
  await expect(panel.getByRole('alert')).toContainText('Фильтр по источнику');
  await expect(panel.getByRole('button', { name: 'Добавить на панель' })).toBeDisabled();
  await panel.getByLabel('Фильтр').fill('');
  await expect(panel.getByRole('button', { name: 'Добавить на панель' })).toBeEnabled();
});

test('конструктор: шестерёнка открывает правку, сохранение меняет название', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Настроить «Норма сбережений»' }).click();
  const panel = page.getByRole('dialog', { name: 'Настройка графика' });
  await expect(panel.getByLabel('Название')).toHaveValue('Норма сбережений');
  await panel.getByLabel('Название').fill('Норма, %');
  await panel.getByRole('button', { name: 'Сохранить' }).click();
  await expect(page.getByRole('region', { name: 'Норма, %' })).toBeVisible();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(9);
});

test('конструктор: серии по показателям и свой период', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'График', exact: true }).click();
  const panel = page.getByRole('dialog', { name: 'Новый график' });
  await panel.getByLabel('Серии').selectOption('metric');
  await expect(panel.getByRole('checkbox', { name: 'Доходы' })).toBeChecked();
  await panel.getByRole('checkbox', { name: 'Сбережения', exact: true }).check();
  await panel.getByLabel('Период', { exact: true }).selectOption('range');
  await expect(panel.getByLabel('С', { exact: true })).toBeVisible();
  await expect(panel.getByRole('img', { name: 'Новый график' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(panel).toBeHidden();
});

test('сценарий 6: график с накоплением по статусам → на панель → перетащить → раскладка сохранена', async ({
  page
}) => {
  // Высокое окно: вся сетка на экране, перетаскивание не упирается в прокрутку.
  await page.setViewportSize({ width: 1440, height: 3200 });
  await open(page);
  await page.getByRole('button', { name: 'График', exact: true }).click();
  const panel = page.getByRole('dialog', { name: 'Новый график' });
  await panel.getByLabel('Название').fill('Статусы по месяцам');
  await panel.getByLabel('Серии').selectOption('status');
  await panel.getByRole('radio', { name: 'С накоплением' }).click();
  await panel.getByRole('button', { name: 'Добавить на панель' }).click();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(10);

  const grip = page.getByRole('button', { name: /Переместить «Статусы по месяцам»/ });
  const first = page.getByRole('region', { name: 'Доходы, расходы и сбережения' });
  const from = await grip.boundingBox();
  const to = await first.boundingBox();
  if (from === null || to === null) throw new Error('нет геометрии');
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + 20, to.y + 20, { steps: 12 });
  await page.mouse.up();
  await expect.poll(async () => (await titles(page))[0]).toBe('Статусы по месяцам');

  // раскладка ушла в базу: после смены вкладки и возврата порядок тот же
  await page.getByRole('button', { name: 'Новый дашборд' }).click();
  await page.getByLabel('Название').fill('Второй');
  await page.getByRole('button', { name: 'Создать' }).click();
  await page.getByRole('tab', { name: 'Мой бюджет' }).click();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(10);
  expect((await titles(page))[0]).toBe('Статусы по месяцам');
});

import { expect, test, type Locator, type Page } from '@playwright/test';

/** Относительная яркость и контраст по WCAG 2.x для цветов `rgb(r, g, b)`. */
function contrast(fg: string, bg: string): number {
  const lum = (css: string): number => {
    const [r = 0, g = 0, b = 0] = (css.match(/\d+(\.\d+)?/g) ?? []).slice(0, 3).map(Number);
    const lin = (c: number): number => {
      const v = c / 255;
      return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  };
  const [a, b] = [lum(fg), lum(bg)];
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

async function colors(button: Locator): Promise<{ bg: string; fg: string }> {
  return button.evaluate((el) => {
    const style = getComputedStyle(el);
    return { bg: style.backgroundColor, fg: style.color };
  });
}

/** Цвет `--primary` в текущей теме, как его считает браузер: все основные действия имеют именно его. */
async function primaryBackground(page: Page): Promise<string> {
  return page.evaluate(() => {
    const probe = document.createElement('span');
    probe.style.backgroundColor = 'var(--primary)';
    document.body.append(probe);
    const value = getComputedStyle(probe).backgroundColor;
    probe.remove();
    return value;
  });
}

for (const theme of ['light', 'dark'] as const) {
  test(`кнопка «+ Добавить категорию» основного цвета и читаемая (${theme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.goto('/categories?vault=open');
    const add = page.getByRole('button', { name: 'Добавить категорию' });
    await expect(add).toBeVisible();
    await expect(add).toContainText('+');
    const { bg, fg } = await colors(add);
    expect(bg).toBe(await primaryBackground(page));
    expect(contrast(fg, bg)).toBeGreaterThanOrEqual(4.5);
  });

  test(`кнопка «+ Новый доход» основного цвета и читаемая (${theme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.goto('/incomes?vault=open');
    const add = page.getByRole('button', { name: /^Новый доход/ }).first();
    await expect(add).toBeVisible();
    await expect(add).toContainText('+');
    const { bg, fg } = await colors(add);
    expect(bg).toBe(await primaryBackground(page));
    expect(contrast(fg, bg)).toBeGreaterThanOrEqual(4.5);
  });
}

test('«+ Добавить категорию» открывает редактор новой категории', async ({ page }) => {
  await page.goto('/categories?vault=open');
  await page.getByRole('button', { name: 'Добавить категорию' }).click();
  await expect(page.getByLabel('Название')).toBeVisible();
});

test('«+ Новый доход» открывает форму дохода', async ({ page }) => {
  await page.goto('/incomes?vault=open');
  await page
    .getByRole('button', { name: /^Новый доход/ })
    .first()
    .click();
  await expect(page.getByRole('dialog', { name: 'Новый доход' })).toBeVisible();
});

test('в пустом месяце «+ Новый доход» в пустом состоянии тоже основного цвета', async ({
  page
}) => {
  await page.goto('/incomes?vault=open');
  const switcher = page.getByRole('button', { name: /Предыдущий месяц/ });
  const empty = page.getByText('В этом месяце доходов нет');
  for (let i = 0; i < 24 && !(await empty.isVisible()); i++) await switcher.click();
  await expect(empty).toBeVisible();
  const add = page.getByRole('main').getByRole('button', { name: /^Новый доход/ });
  await expect(add).toHaveCount(2);
  const { bg, fg } = await colors(add.last());
  expect(bg).toBe(await primaryBackground(page));
  expect(contrast(fg, bg)).toBeGreaterThanOrEqual(4.5);
  await expect(add.last()).toContainText('+');
});

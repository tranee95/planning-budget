import { expect, test, type Page } from '@playwright/test';
import { openSeptember } from './september';
import { openQuickAdd } from './quick-add';

type Field = { name: string; placeholder: string; text: number; avail: number };

/** Видимые текстовые поля и размеры их подсказок (ширина текста по canvas — без обрезки). */
async function fields(page: Page): Promise<Field[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll('input,textarea')]
      .filter((el) => {
        const i = el as HTMLInputElement;
        const r = el.getBoundingClientRect();
        return (
          r.width > 0 && r.height > 0 && !['checkbox', 'radio', 'date', 'password'].includes(i.type)
        );
      })
      .map((el) => {
        const i = el as HTMLInputElement;
        const style = getComputedStyle(i);
        const ctx = document.createElement('canvas').getContext('2d');
        if (ctx) ctx.font = `${style.fontSize} ${style.fontFamily}`;
        return {
          name: i.getAttribute('aria-label') ?? i.id,
          placeholder: i.placeholder,
          text: ctx?.measureText(i.placeholder).width ?? 0,
          avail: i.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight)
        };
      })
  );
}

async function expectGood(page: Page, scene: string): Promise<void> {
  await page.locator('input:visible, textarea:visible').first().waitFor();
  const list = await fields(page);
  expect(list.length, scene).toBeGreaterThan(0);
  for (const f of list) {
    expect(f.placeholder, `${scene}: у поля «${f.name}» нет подсказки`).not.toBe('');
    expect(f.text, `${scene}: подсказка поля «${f.name}» обрезана`).toBeLessThanOrEqual(f.avail);
  }
}

for (const width of [1024, 1440]) {
  test.describe(`подсказки в полях ввода ${String(width)} px`, () => {
    test.use({ viewport: { width, height: 800 } });

    test('расходы: строка в карточке', async ({ page }) => {
      await openSeptember(page);
      await page.getByRole('button', { name: /Добавить трату в Продукты/ }).click();
      await expectGood(page, 'строка в карточке');
    });

    test('расходы: быстрое добавление', async ({ page }) => {
      await openQuickAdd(page);
      await expectGood(page, 'быстрое добавление');
    });

    test('расходы → таблица и доходы: поиск', async ({ page }) => {
      await page.goto('/expenses?vault=open');
      await page.getByRole('radio', { name: 'Таблица' }).click();
      await expectGood(page, 'поиск по тратам');
      await page.goto('/incomes?vault=open');
      await expectGood(page, 'поиск по доходам');
    });

    test('настройки и правка строки таблицы', async ({ page }) => {
      await page.goto('/settings?vault=open');
      await expectGood(page, 'настройки');
      await page.goto('/expenses?vault=open');
      const prev = page.getByRole('button', { name: /Предыдущий месяц/ });
      for (let i = 0; i < 12 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
        await prev.click();
      }
      await page.getByRole('button', { name: /Магазин у дома/ }).click();
      await expectGood(page, 'правка строки');
    });

    test('доходы: новый доход', async ({ page }) => {
      await page.goto('/incomes?vault=open');
      await page
        .getByRole('button', { name: /^Новый доход/ })
        .first()
        .click();
      await expect(page.getByRole('dialog', { name: 'Новый доход' })).toBeVisible();
      await expectGood(page, 'новый доход');
    });

    test('долги, категории, сбережения', async ({ page }) => {
      await page.goto('/debts?vault=open');
      await page.getByRole('button', { name: '+ Долг' }).click();
      await expect(page.getByRole('dialog')).toBeVisible();
      await expectGood(page, 'долг');
      await page.keyboard.press('Escape');
      await page.goto('/categories?vault=open');
      await page.getByRole('button', { name: 'Добавить категорию' }).click();
      await expectGood(page, 'редактор категории');
      await page.goto('/savings?vault=open');
      await expectGood(page, 'сбережения');
    });
  });
}

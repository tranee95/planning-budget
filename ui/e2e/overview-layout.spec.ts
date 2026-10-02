// Экран «Обзор» на разных вьюпортах и масштабах, пустой и заполненный месяц: ничего не выходит
// за окно, ключевые блоки видны, на пустом графике нет повторяющихся подписей оси Y.
import { expect, test, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const VIEWPORTS = [
  [960, 640, 'минимальное окно'],
  [1024, 768, 'маленький ноутбук'],
  [1280, 720, '1920×1080 при 150%'],
  [1366, 768, 'типичный ноутбук'],
  [1536, 864, '1920×1080 при 125%'],
  [1920, 1080, 'Full HD'],
  [2560, 1440, '2K']
] as const;
const SCALES = [1, 1.25, 1.5];
const OUT = join(process.cwd(), '..', 'ui-test-artifacts', 'overview');

type Data = 'empty' | 'filled';

// В моках данные лежат в сентябре 2026; приложение открывается на текущем месяце (пустом).
async function openOverview(page: Page, data: Data): Promise<void> {
  await page.goto('/?vault=open');
  await page.getByRole('heading', { level: 1, name: 'Обзор' }).waitFor();
  if (data === 'filled') {
    const prev = page.getByRole('button', { name: /Предыдущий месяц/ });
    for (
      let i = 0;
      i < 24 && !(await page.locator('.topbar').getByText('Сентябрь 2026').isVisible());
      i++
    ) {
      await prev.click();
    }
    await expect(page.locator('.topbar').getByText('Сентябрь 2026')).toBeVisible();
  }
  await page.getByText('Лимиты месяца').waitFor();
}

type Measure = {
  scrollWidth: number;
  innerWidth: number;
  boxes: { name: string; left: number; right: number; width: number }[];
  yLabels: string[];
};

function measure(page: Page): Promise<Measure> {
  return page.evaluate(() => {
    const box = (name: string, el: Element | null) => {
      const r = el?.getBoundingClientRect();
      return { name, left: r?.left ?? -1, right: r?.right ?? -1, width: r?.width ?? 0 };
    };
    const card = (title: string) =>
      [...document.querySelectorAll('section')].find((s) => s.textContent.includes(title)) ?? null;
    const chips = [...document.querySelectorAll('.strip .chips > *')].map((el, i) =>
      box(`чип статуса ${String(i + 1)}`, el)
    );
    const chart = card('Доходы, расходы и сбережения');
    const yLabels = [...(chart?.querySelectorAll('svg text[text-anchor="end"]') ?? [])].map((t) =>
      t.textContent.trim()
    );
    return {
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth: window.innerWidth,
      boxes: [
        box('поиск', document.querySelector('.topbar .search')),
        ...chips,
        box('график', chart),
        box('лимиты месяца', card('Лимиты месяца')),
        box('сводка', document.querySelector('.screen'))
      ],
      yLabels
    };
  });
}

mkdirSync(OUT, { recursive: true });

for (const scale of SCALES) {
  test.describe(`масштаб ${String(scale)}`, () => {
    test.use({ deviceScaleFactor: scale });
    for (const data of ['empty', 'filled'] as Data[]) {
      for (const [w, h, label] of VIEWPORTS) {
        test(`${data === 'empty' ? 'пустой' : 'заполненный'} месяц · ${String(w)}×${String(h)} (${label})`, async ({
          page
        }) => {
          await page.setViewportSize({ width: w, height: h });
          await openOverview(page, data);
          await page.waitForTimeout(400);
          await page.screenshot({
            path: join(OUT, `${data}-${String(w)}x${String(h)}-x${String(scale)}.png`)
          });
          const m = await measure(page);

          expect(m.scrollWidth, 'горизонтальное переполнение страницы').toBeLessThanOrEqual(
            m.innerWidth
          );
          for (const b of m.boxes) {
            expect(b.width, `${b.name}: нулевая ширина`).toBeGreaterThan(0);
            expect(b.left, `${b.name}: левее окна`).toBeGreaterThanOrEqual(0);
            expect(b.right, `${b.name}: правее окна`).toBeLessThanOrEqual(m.innerWidth + 0.5);
          }
          expect(new Set(m.yLabels).size, `подписи оси Y повторяются: ${m.yLabels.join(' ')}`).toBe(
            m.yLabels.length
          );
        });
      }
    }
  });
}

test('сбережения без дохода: шкала коридора не рисуется, с доходом — рисуется', async ({
  page
}) => {
  await openOverview(page, 'empty');
  await expect(page.getByText('нет дохода в месяце')).toBeVisible();
  await expect(page.getByRole('img', { name: /^Сбережения .* %,/ })).toHaveCount(0);

  await openOverview(page, 'filled');
  await expect(page.getByRole('img', { name: /^Сбережения .* %,/ })).toBeVisible();
});

import { expect, test, type Page } from '@playwright/test';

// Кольцо фокуса рисует обёртка поля, а не внутренний input.
async function style(
  page: Page,
  selector: string,
  props: string[]
): Promise<Record<string, string>> {
  return page
    .locator(selector)
    .first()
    .evaluate((el, names) => {
      const s = getComputedStyle(el);
      return Object.fromEntries(names.map((n) => [n, s.getPropertyValue(n)]));
    }, props);
}

for (const theme of ['light', 'dark'] as const) {
  test(`поле пароля (${theme}): кольцо на обёртке, у внутреннего input нет outline`, async ({
    page
  }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.goto('/lock');
    await page.getByLabel('Пароль', { exact: true }).click();
    const input = await style(page, '#password', ['outline-style', 'box-shadow']);
    expect(input['outline-style']).toBe('none');
    expect(input['box-shadow']).toBe('none');
    const box = await style(page, '#password >> xpath=..', ['box-shadow', 'border-top-color']);
    expect(box['box-shadow']).not.toBe('none');
  });

  test(`палитра (${theme}): у поля поиска нет ни рамки, ни кольца`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.goto('/?vault=open');
    await page.getByRole('heading', { level: 1 }).waitFor();
    await page.keyboard.press('Control+KeyK');
    const field = page.getByRole('dialog', { name: 'Поиск и команды' }).locator('input');
    await field.waitFor();
    const s = await field.evaluate((el) => {
      const c = getComputedStyle(el);
      return { outline: c.outlineStyle, shadow: c.boxShadow, border: c.borderTopWidth };
    });
    expect(s).toEqual({ outline: 'none', shadow: 'none', border: '0px' });
  });
}

test('кнопка: клик мышью не оставляет кольца, Tab — оставляет', async ({ page }) => {
  await page.goto('/?vault=open');
  const next = page.getByRole('button', { name: 'Следующий месяц' });
  await next.click();
  expect(await next.evaluate((el) => getComputedStyle(el).boxShadow)).not.toMatch(
    /rgb.*0px 0px 0px 4px/
  );
  await page.keyboard.press('Tab');
  await page.keyboard.press('Shift+Tab');
  await expect(next).toBeFocused();
  expect(await next.evaluate((el) => getComputedStyle(el).boxShadow)).toMatch(/0px 0px 0px 4px/);
});

test('ошибка + фокус: красное кольцо вместо основного, размер поля не меняется', async ({
  page
}) => {
  await page.goto('/lock');
  const input = page.getByLabel('Пароль', { exact: true });
  const wrap = page.locator('#password >> xpath=..');
  const before = await wrap.boundingBox();
  await input.fill('wrong-password');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Неверный пароль')).toBeVisible();
  await input.click();
  const after = await wrap.boundingBox();
  expect(after?.width).toBe(before?.width);
  expect(after?.height).toBe(before?.height);
  const ring = await wrap.evaluate((el) => getComputedStyle(el).boxShadow);
  const danger = await page.evaluate(() => {
    const probe = document.createElement('span');
    probe.style.color = 'var(--danger-ring)';
    document.body.append(probe);
    const c = getComputedStyle(probe).color;
    probe.remove();
    return c;
  });
  expect(ring).toContain(danger);
});

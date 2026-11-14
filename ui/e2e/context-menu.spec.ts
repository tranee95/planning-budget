import { expect, test, type Page } from '@playwright/test';

async function openSeptember(page: Page): Promise<void> {
  await page.goto('/expenses?vault=open');
  const switcher = page.getByRole('button', { name: /Предыдущий месяц/ });
  for (let i = 0; i < 12 && !(await page.getByText('Сентябрь 2026').isVisible()); i++) {
    await switcher.click();
  }
  await expect(page.getByText('Сентябрь 2026')).toBeVisible();
}

test('правый клик по трате: меню, статус, теги, край окна, клавиатура и возврат фокуса', async ({
  page
}) => {
  await openSeptember(page);
  const row = page.getByRole('listitem').filter({ hasText: 'Кино' });
  const menu = page.getByRole('menu', { name: 'Действия с тратой' });

  await test.step('меню открывается у курсора, «Выбрать» в блоках нет', async () => {
    await row.click({ button: 'right' });
    await expect(menu).toBeVisible();
    await expect(menu.getByRole('menuitem', { name: 'Изменить' })).toBeFocused();
    await expect(menu.getByRole('menuitem', { name: 'Теги…' })).toBeVisible();
    await expect(menu.getByRole('menuitem', { name: /Выбрать/ })).toHaveCount(0);
    await expect(menu.getByRole('menuitemradio')).toHaveCount(4);
  });

  await test.step('смена статуса из меню закрывает его', async () => {
    await menu.getByRole('menuitemradio', { name: 'План', exact: true }).click();
    await expect(menu).toBeHidden();
    await expect(row.getByRole('button', { name: /^План/ })).toBeVisible();
  });

  await test.step('«Теги…» открывает окно выбора тегов', async () => {
    await row.click({ button: 'right' });
    await menu.getByRole('menuitem', { name: 'Теги…' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeHidden();
  });

  await test.step('у края окна меню не выходит за него, Esc закрывает', async () => {
    const size = page.viewportSize();
    if (!size) throw new Error('нет размера окна');
    await row.scrollIntoViewIfNeeded();
    const box = await row.boundingBox();
    if (!box) throw new Error('нет строки');
    await page.mouse.click(box.x + box.width - 2, box.y + box.height / 2, { button: 'right' });
    const rect = await menu.boundingBox();
    if (rect) {
      expect(rect.x + rect.width).toBeLessThanOrEqual(size.width);
      expect(rect.y + rect.height).toBeLessThanOrEqual(size.height);
    }
    await page.keyboard.press('Escape');
    await expect(menu).toBeHidden();
  });

  await test.step('с клавиатуры: клавиша меню открывает, Esc возвращает фокус', async () => {
    const status = row.getByRole('button', { name: /Сменить статус/ });
    await status.focus();
    await page.keyboard.press('ContextMenu');
    await expect(menu).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(menu).toBeHidden();
    await expect(status).toBeFocused();
  });
});

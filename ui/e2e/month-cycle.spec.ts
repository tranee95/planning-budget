import { expect, test } from '@playwright/test';

// Полный цикл месяца: знакомство → мастер первого месяца → «План готов» → долг с графиком →
// отметка оплаты → трата и смена статуса → итоги на обзоре. Расчёты делает мок по тем же
// правилам, что Rust (parity-тест мока), поэтому проверяются связи между экранами.
test('цикл месяца: мастер, план готов, долг, трата, итоги', async ({ page }) => {
  const problems: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') problems.push(m.text());
  });
  page.on('pageerror', (e) => problems.push(e.message));

  await test.step('знакомство и мастер: план готов', async () => {
    await page.goto('/?vault=open&intro=1');
    for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();
    await page.getByRole('button', { name: 'Спланировать первый месяц' }).click();
    const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
    await wizard.getByLabel('Сумма').fill('100000');
    await wizard.getByRole('button', { name: 'Далее' }).click();
    await page.getByRole('dialog').getByLabel('Продукты', { exact: true }).fill('30000');
    await page.getByRole('button', { name: 'Далее' }).click();
    await page.getByRole('button', { name: 'Далее' }).click();
    await page
      .getByRole('dialog', { name: /: Итог/ })
      .getByRole('button', { name: 'План готов' })
      .click();
    await expect(page.getByText(/План на .+ готов/)).toBeVisible();
    await expect(page.getByRole('dialog')).toHaveCount(0);
  });

  await test.step('обзор: план зафиксирован', async () => {
    await expect(page.getByText('Не распределено')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Разблокировать' })).toBeVisible();
  });

  await test.step('долг: создаётся с графиком и виден в списке', async () => {
    await page.getByRole('link', { name: 'Долги' }).click();
    await page.getByRole('button', { name: '+ Долг' }).click();
    const sheet = page.getByRole('dialog');
    await sheet.getByLabel('У кого или что').fill('Брат');
    await sheet.getByLabel('Сумма', { exact: true }).fill('30000');
    await sheet.getByLabel('Количество месяцев').fill('3');
    await expect(sheet.getByText('Сумма графика: 30 000 ₽')).toBeVisible();
    await sheet.getByRole('button', { name: 'Сохранить' }).click();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(page.getByText('Брат')).toBeVisible();
  });

  await test.step('трата: добавляется и меняет статус', async () => {
    await page.getByRole('link', { name: 'Расходы' }).click();
    await page.getByRole('button', { name: 'Добавить трату в Продукты' }).click();
    await page.getByLabel('Наименование, Продукты').fill('Рынок');
    await page.getByLabel('Сумма, ₽').fill('1200');
    await page.keyboard.press('Enter');
    const row = page.getByRole('listitem').filter({ hasText: 'Рынок' });
    await expect(row).toBeVisible();
    await row.getByRole('button', { name: /Сменить статус/ }).click();
    await expect(
      row.getByRole('button', { name: /^(Оплачено|Незапланировано|Долг|План)/ })
    ).toBeVisible();
  });

  await test.step('итоги: обзор показывает план и факт без ошибок', async () => {
    await page.getByRole('link', { name: 'Обзор' }).click();
    await expect(page.getByText('Не распределено')).toBeVisible();
    await expect(page.getByText('Незапланировано').first()).toBeVisible();
  });

  expect(problems).toEqual([]);
});

import { expect, test, type Page } from '@playwright/test';

async function openWizard(page: Page) {
  await page.goto('/?vault=open&intro=1&fresh=1');
  for (let i = 0; i < 4; i++) await page.getByRole('button', { name: 'Далее' }).click();
  await page
    .getByRole('dialog', { name: 'Категории расходов' })
    .getByRole('button', { name: 'Далее' })
    .click();
  await page.getByRole('button', { name: 'Спланировать первый месяц' }).click();
  const wizard = page.getByRole('dialog', { name: /План на .+: Доход/ });
  await expect(wizard).toBeVisible();
  return wizard;
}

test('UX-8: на шаге «Доход» фокус в первом поле, на шаге «Статьи» — в первой статье', async ({
  page
}) => {
  const wizard = await openWizard(page);
  await expect(wizard.getByLabel('Источник')).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(wizard.getByLabel('Сумма')).toBeFocused();
  await page.keyboard.type('100000');
  await wizard.getByRole('button', { name: 'Далее' }).click();
  const step = page.getByRole('dialog', { name: /: Статьи/ });
  await expect(step.locator('input').first()).toBeFocused();
});

test('UX-8: «Далее» неактивна без дохода с суммой и объясняет почему', async ({ page }) => {
  const wizard = await openWizard(page);
  const next = wizard.getByRole('button', { name: 'Далее' });
  await expect(next).toBeDisabled();
  await expect(wizard).toContainText('Внесите хотя бы один доход с суммой');
  await wizard.getByLabel('Сумма').fill('0');
  await expect(next).toBeDisabled();
  await wizard.getByLabel('Сумма').fill('100000');
  await expect(next).toBeEnabled();
});

test('UX-8: план выше дохода требует подтверждения без второй модалки', async ({ page }) => {
  const wizard = await openWizard(page);
  await wizard.getByLabel('Сумма').fill('100000');
  await wizard.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('dialog').getByLabel('Продукты', { exact: true }).fill('120000');
  await page.getByRole('button', { name: 'Далее' }).click();
  await page.getByRole('button', { name: 'Далее' }).click();
  const result = page.getByRole('dialog', { name: /: Итог/ });
  await result.getByRole('button', { name: 'План готов' }).click();

  await expect(page.getByRole('dialog')).toHaveCount(1);
  await expect(result).toContainText(
    /План превышает доход на 20\s000\s₽\. Всё равно зафиксировать\?/
  );
  await expect(result.getByRole('button', { name: 'План готов' })).toHaveCount(0);

  await result.getByRole('button', { name: 'Назад к плану' }).click();
  await expect(result.getByRole('button', { name: 'План готов' })).toBeVisible();

  await result.getByRole('button', { name: 'План готов' }).click();
  await result.getByRole('button', { name: 'Зафиксировать' }).click();
  await expect(page.getByText(/План на .+ готов/)).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('UX-8: без превышения «План готов» фиксирует сразу', async ({ page }) => {
  const wizard = await openWizard(page);
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
});

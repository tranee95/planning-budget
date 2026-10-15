import { expect, test } from '@playwright/test';

test('сбой загрузки экрана: «500 Internal Error» и запись в журнал приложения', async ({
  page
}) => {
  await page.goto('/?vault=open');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  // Перехват IPC: записываем вызовы команды журнала, остальное идёт в мок.
  await page.evaluate(() => {
    const w = window as unknown as {
      __logged: unknown[];
      __TAURI_INTERNALS__: { invoke: (...args: unknown[]) => unknown };
    };
    w.__logged = [];
    const original = w.__TAURI_INTERNALS__.invoke;
    w.__TAURI_INTERNALS__.invoke = (cmd: unknown, args: unknown, options: unknown) => {
      if (cmd === 'client_error_log') w.__logged.push(args);
      return original(cmd, args, options);
    };
  });
  await page.route(/analytics\/\+page\.svelte/, (route) =>
    route.fulfill({ status: 500, contentType: 'text/javascript', body: 'throw new Error("x")' })
  );
  await page.getByRole('link', { name: 'Аналитика' }).click();
  await expect(page.getByText('Internal Error')).toBeVisible();

  const logged = (await page.evaluate(
    () => (window as unknown as { __logged: { entry: Record<string, string> }[] }).__logged
  )) as { entry: Record<string, string> }[];
  expect(logged).toHaveLength(1);
  expect(logged[0]?.entry).toMatchObject({
    kind: 'route',
    name: 'TypeError',
    route: '/analytics'
  });
  expect(logged[0]?.entry.message).toContain('Failed to fetch dynamically imported module');
});

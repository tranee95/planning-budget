import { defineConfig } from '@playwright/test';

// Порт dev-сервера: UI_PORT позволяет вести проверки в двух рабочих копиях одновременно.
const PORT = Number(process.env.UI_PORT ?? 1421);
const BASE = `http://127.0.0.1:${String(PORT)}`;

export default defineConfig({
  testDir: 'e2e',
  testIgnore: 'qa-*.spec.ts', // тяжёлая матрица запускается отдельно: pnpm test:ui
  use: {
    baseURL: BASE,
    viewport: { width: 1440, height: 900 },
    reducedMotion: 'reduce'
  },
  webServer: {
    command: `pnpm exec vite dev --mode mock --port ${String(PORT)} --strictPort`,
    url: BASE,
    reuseExistingServer: true
  }
});

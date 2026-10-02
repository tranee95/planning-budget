import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'e2e',
  testIgnore: 'qa-*.spec.ts', // тяжёлая матрица запускается отдельно: pnpm test:ui
  use: {
    baseURL: 'http://127.0.0.1:1421',
    viewport: { width: 1440, height: 900 },
    reducedMotion: 'reduce'
  },
  webServer: {
    command: 'pnpm dev:mock',
    url: 'http://127.0.0.1:1421',
    reuseExistingServer: true
  }
});

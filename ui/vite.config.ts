import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: { host: '127.0.0.1', strictPort: true },
  // Сканируем все компоненты на старте: иначе зависимости (иконки lucide) находятся лениво на
  // первом заходе на экран, Vite пересобирает кеш посреди перехода и первый клик не срабатывает.
  // Тесты и test-setup исключены: иначе в dev-сборку зависимостей попадают vitest и testing-library.
  optimizeDeps: {
    entries: ['src/**/*.{svelte,ts}', '!src/**/*.{test,spec}.ts', '!src/test-setup.ts']
  }
});

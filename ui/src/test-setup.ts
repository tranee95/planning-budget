import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// jsdom не реализует ResizeObserver, а Svelte использует его для bind:clientHeight.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
);

// jsdom не реализует matchMedia: svelte/motion создаёт MediaQuery при импорте.
vi.stubGlobal('matchMedia', (query: string) => ({
  matches: false,
  media: query,
  addEventListener: () => undefined,
  removeEventListener: () => undefined
}));

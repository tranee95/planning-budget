import { afterEach, describe, expect, it } from 'vitest';
import { lockScroll } from './overlay';

afterEach(() => {
  document.documentElement.removeAttribute('style');
});

describe('lockScroll', () => {
  it('locks the root once for nested overlays and restores it after the last one', () => {
    const root = document.documentElement;
    root.style.scrollbarGutter = 'stable';
    const first = lockScroll();
    const second = lockScroll();
    expect(root.style.overflow).toBe('hidden');
    expect(root.style.scrollbarGutter).toBe('auto');
    first();
    expect(root.style.overflow).toBe('hidden');
    second();
    expect(root.style.overflow).toBe('');
    expect(root.style.scrollbarGutter).toBe('stable');
  });

  it('ignores a repeated release', () => {
    const release = lockScroll();
    const other = lockScroll();
    release();
    release();
    expect(document.documentElement.style.overflow).toBe('hidden');
    other();
    expect(document.documentElement.style.overflow).toBe('');
  });
});

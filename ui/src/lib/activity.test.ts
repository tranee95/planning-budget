import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { startActivityPing } from './activity';

let pings = 0;
let clock = 0;

beforeEach(() => {
  pings = 0;
  clock = 1_000_000;
  mockIPC((cmd) => {
    if (cmd === 'activity_ping') pings += 1;
    return null;
  });
});

afterEach(() => {
  clearMocks();
});

test('первое действие шлёт ping сразу, дальше не чаще раза в 30 с', async () => {
  const stop = startActivityPing(window, () => clock);
  window.dispatchEvent(new Event('pointerdown'));
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));
  clock += 29_000;
  window.dispatchEvent(new Event('pointerdown'));
  await vi.waitFor(() => {
    expect(pings).toBe(1);
  });

  clock += 2_000;
  window.dispatchEvent(new Event('pointerdown'));
  await vi.waitFor(() => {
    expect(pings).toBe(2);
  });
  stop();
});

test('после отписки события игнорируются', () => {
  const stop = startActivityPing(window, () => clock);
  stop();
  window.dispatchEvent(new Event('pointerdown'));
  expect(pings).toBe(0);
});

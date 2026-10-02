import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, expect, test } from 'vitest';
import { commands } from '../bindings';
import { installMocks } from './install';

afterEach(() => {
  clearMocks();
});

test('сгенерированная команда проходит через мок IPC', async () => {
  installMocks();
  const result = await commands.appVersion();
  expect(result).toEqual({ status: 'ok', data: '0.0.0-mock' });
});

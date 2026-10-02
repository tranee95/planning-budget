import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { handlers } from './handlers';

/** Подменяет IPC Tauri обработчиками из `handlers`. Подключать только динамическим импортом под VITE_MOCK. */
export function installMocks(): void {
  mockWindows('main');
  mockIPC(
    (cmd, args) => {
      const handler = (handlers as Record<string, ((args: unknown) => unknown) | undefined>)[cmd];
      if (!handler) throw new Error(`mock: нет обработчика ${cmd}`);
      return handler(args);
    },
    { shouldMockEvents: true }
  );
}

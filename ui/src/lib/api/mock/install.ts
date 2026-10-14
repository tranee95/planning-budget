import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { resultCache } from '../result-cache';
import { handlers } from './handlers';

/** Команды, которые только читают: остальные считаются записью и сбрасывают кеш расчётов. */
const READ =
  /(^|_)(list|get|month|year|series|run|run_many|check|progress|overview|preview|search|suggest|usage|defaults|history|parse|version|dark|ping)$/;

const READ_EXACT = new Set(['vault_status', 'query_parse', 'search', 'app_version']);

export const isReadCommand = (cmd: string): boolean => READ_EXACT.has(cmd) || READ.test(cmd);

/**
 * Подменяет IPC Tauri обработчиками из `handlers`. Подключать только динамическим импортом под VITE_MOCK.
 * Настоящий Rust сообщает об изменении событием `data-changed`, которое сбрасывает кеш расчётов
 * (`lib/api/result-cache`); мок событий не шлёт, поэтому запись сбрасывает кеш прямо здесь.
 */
export function installMocks(): void {
  mockWindows('main');
  mockIPC(
    (cmd, args) => {
      const handler = (handlers as Record<string, ((args: unknown) => unknown) | undefined>)[cmd];
      if (!handler) throw new Error(`mock: нет обработчика ${cmd}`);
      if (!isReadCommand(cmd)) resultCache.clear();
      return handler(args);
    },
    { shouldMockEvents: true }
  );
}

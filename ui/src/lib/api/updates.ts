import type { UpdateStatusDto } from './bindings';
import { commands } from './bindings';
import { call } from './call';

export const updatesApi = {
  /** `manual = false` — автопроверка при запуске: тумблер и не чаще раза в сутки. */
  check: (manual: boolean): Promise<UpdateStatusDto> => call(commands.updateCheck(manual)),
  /** Скачивает, проверяет подпись, закрывает сессию, ставит и перезапускает приложение. */
  install: (): Promise<null> => call(commands.updateInstall())
};

import type { Prefs, PrefsPatch } from './bindings';
import { commands } from './bindings';
import { call } from './call';

export const prefsApi = {
  get: (): Promise<Prefs> => call(commands.prefsGet()),
  set: (patch: PrefsPatch): Promise<Prefs> => call(commands.prefsSet(patch)),
  /** Тема окна: в режиме «системная» — тема ОС. */
  systemDark: (): Promise<boolean> => call(commands.systemDark())
};

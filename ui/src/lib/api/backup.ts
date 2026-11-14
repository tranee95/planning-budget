import { commands } from './bindings';
import { call } from './call';

/** Несколько хранилищ: список и переключение. */
export const vaultsApi = {
  list: () => call(commands.vaultsList()),
  add: (label: string) => call(commands.vaultsAdd(label)),
  switchTo: (id: string) => call(commands.vaultsSwitch(id)),
  rename: (id: string, label: string) => call(commands.vaultsRename(id, label)),
  remove: (id: string) => call(commands.vaultsRemove(id))
};

/** Резервные копии: каталог и файл выбираются нативным диалогом на стороне Rust. */
export const backupApi = {
  status: () => call(commands.backupStatus()),
  create: () => call(commands.backupCreate()),
  pickDir: () => call(commands.backupPickDir()),
  resetDir: () => call(commands.backupResetDir()),
  importNamed: (fileName: string, password: string, label: string) =>
    call(commands.backupImportNamed(fileName, password, label)),
  importPick: (password: string, label: string) => call(commands.backupImportPick(password, label))
};

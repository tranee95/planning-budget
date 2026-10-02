import { commands } from './bindings';
import { call } from './call';

export const vaultApi = {
  status: () => call(commands.vaultStatus()),
  create: (password: string) => call(commands.vaultCreate(password)),
  unlock: (password: string) => call(commands.vaultUnlock(password)),
  unlockRecovery: (code: string, newPassword: string) =>
    call(commands.vaultUnlockRecovery(code, newPassword)),
  changePassword: (oldPassword: string, newPassword: string) =>
    call(commands.vaultChangePassword(oldPassword, newPassword)),
  rekey: (password: string) => call(commands.vaultRekey(password)),
  lock: () => call(commands.vaultLock()),
  reset: (password: string, confirmPhrase: string) =>
    call(commands.vaultReset(password, confirmPhrase)),
  saveRecoveryCode: (code: string) => call(commands.vaultSaveRecoveryCode(code)),
  ping: () => call(commands.activityPing())
};

import type { AppError, Prefs, PrefsPatch, VaultStatusDto } from '../bindings';
import { dashboardHandlers } from './dashboards';
import {
  budgetHandlers,
  mockPlanRateBp,
  mockSavingsCategories,
  mockTransactionById
} from './budget';
import { createSavingsHandlers } from './savings';
import { createDebtHandlers } from './debts';

/**
 * Обработчики моков IPC: один источник для dev:mock, unit, e2e и ui-shot.
 * Логика сейфа здесь упрощена и повторяет только наблюдаемое поведение бэкенда:
 * неверный пароль → WrongPassword, после третьей подряд — задержка.
 */

const MOCK_PASSWORD_REJECTED = 'wrong-password';
const MOCK_RECOVERY_CODE = 'K7QM-2XPA-ZTRB-4HNW-6DVC-ESJF-3Q';
const MOCK_RETRY_MS = 3000;

// ?vault=new — хранилища нет; ?vault=open — сразу разблокировано (для скриншотов и ручной проверки).
const vaultParam = new URLSearchParams(globalThis.location.search).get('vault');
// Знакомство в моке выключено, чтобы не перекрывать экраны в e2e и скриншотах: ?intro=1 включает.
const introParam = new URLSearchParams(globalThis.location.search).get('intro');

const vault = {
  exists: vaultParam !== 'new',
  locked: vaultParam !== 'open',
  failed: 0,
  retryUntil: 0
};

/** Возвращает мок сейфа в исходное состояние: хранилище есть, закрыто, задержки нет (для тестов). */
export function resetMockVault(exists = true): void {
  vault.exists = exists;
  vault.locked = true;
  vault.failed = 0;
  vault.retryUntil = 0;
}

let prefs: Prefs = {
  theme: 'system',
  locale: 'ru-RU',
  uiScale: 100,
  reducedMotion: 'system',
  autolockMinutes: 5,
  introDone: introParam !== '1'
};

function reject(error: AppError): Promise<never> {
  // IPC отдаёт AppError значением, а не Error: bindings.typedError пробрасывает только Error
  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
  return Promise.reject(error);
}

function retryAfterMs(): number {
  return Math.max(0, vault.retryUntil - Date.now());
}

function checkPassword(password: string): Promise<null> {
  if (retryAfterMs() > 0) {
    return reject({ code: 'TooManyAttempts', retryAfterMs: retryAfterMs() });
  }
  if (password === MOCK_PASSWORD_REJECTED) {
    vault.failed += 1;
    if (vault.failed >= 3) vault.retryUntil = Date.now() + MOCK_RETRY_MS;
    return reject({ code: 'WrongPassword', retryAfterMs: retryAfterMs() });
  }
  vault.failed = 0;
  return Promise.resolve(null);
}

export const handlers = {
  ...budgetHandlers,
  ...dashboardHandlers,
  ...createDebtHandlers(mockTransactionById),
  ...createSavingsHandlers(mockSavingsCategories, mockPlanRateBp),
  app_version: () => '0.0.0-mock',

  vault_status: (): VaultStatusDto => ({
    exists: vault.exists,
    locked: vault.locked,
    retryAfterMs: retryAfterMs()
  }),
  vault_create: (args: { password: string }) => {
    if (Array.from(args.password).length < 10) {
      return reject({
        code: 'Validation',
        messageKey: 'errors.vault.password_too_short',
        field: 'password'
      });
    }
    vault.exists = true;
    vault.locked = false;
    return { recoveryCode: MOCK_RECOVERY_CODE };
  },
  vault_unlock: async (args: { password: string }) => {
    await checkPassword(args.password);
    vault.locked = false;
    return null;
  },
  vault_unlock_recovery: (args: { code: string; newPassword: string }) => {
    const normalized = args.code.replace(/[\s-]/g, '').toUpperCase();
    if (normalized !== MOCK_RECOVERY_CODE.replace(/-/g, '')) {
      return reject({
        code: 'Validation',
        messageKey: 'errors.vault.wrong_recovery_code',
        field: 'code'
      });
    }
    vault.locked = false;
    vault.failed = 0;
    vault.retryUntil = 0;
    return null;
  },
  vault_change_password: async (args: { oldPassword: string }) => {
    await checkPassword(args.oldPassword);
    return null;
  },
  vault_rekey: async (args: { password: string }) => {
    await checkPassword(args.password);
    return { recoveryCode: MOCK_RECOVERY_CODE };
  },
  vault_lock: () => {
    vault.locked = true;
    return null;
  },
  vault_reset: () => {
    vault.exists = false;
    vault.locked = true;
    return null;
  },
  vault_save_recovery_code: () => ({ saved: true }),
  activity_ping: () => null,

  prefs_get: () => prefs,
  // В браузере тему ОС отдаёт media-запрос (в окне Tauri её сообщает Rust).
  system_dark: () => globalThis.matchMedia('(prefers-color-scheme: dark)').matches,
  prefs_set: (args: { patch: PrefsPatch }) => {
    const { theme, locale, uiScale, reducedMotion, introDone } = args.patch;
    prefs = {
      theme: theme ?? prefs.theme,
      locale: locale ?? prefs.locale,
      uiScale: uiScale ?? prefs.uiScale,
      reducedMotion: reducedMotion ?? prefs.reducedMotion,
      autolockMinutes: prefs.autolockMinutes,
      introDone: introDone ?? prefs.introDone
    };
    return prefs;
  }
} satisfies Record<string, (args: never) => unknown>;

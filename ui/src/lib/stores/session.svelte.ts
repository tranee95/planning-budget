import { ApiError, onBackendLocked } from '$lib/api/call';
import { vaultApi } from '$lib/api/vault';

/** booting — ждём `vault_status`; setup — хранилища нет; locked — есть, закрыто; unlocked — открыто. */
export type Phase = 'booting' | 'setup' | 'locked' | 'unlocked';

const resetters: Array<() => void> = [];

/** Сторы с данными регистрируют сюда очистку: при блокировке в памяти не остаётся ничего. */
export function onLock(reset: () => void): void {
  resetters.push(reset);
}

/** Куда вести пользователя при текущей фазе; `null` — оставить как есть. */
export function routeFor(
  phase: Phase,
  pathname: string,
  recoveryPending: boolean
): '/' | '/lock' | null {
  const onLockScreen = pathname === '/lock' || pathname.startsWith('/lock/');
  switch (phase) {
    case 'booting':
      return null;
    case 'setup':
    case 'locked':
      return onLockScreen ? null : '/lock';
    case 'unlocked':
      // Экран с recovery-кодом (он живёт на /lock) показывается, пока пользователь
      // его не подтвердил: и после создания хранилища, и после перевыпуска ключа.
      if (recoveryPending) return onLockScreen ? null : '/lock';
      return onLockScreen ? '/' : null;
  }
}

class SessionStore {
  phase = $state<Phase>('booting');
  /** Момент (мс, Date.now), до которого вход закрыт задержкой после неверных паролей. */
  retryUntil = $state(0);
  /** Код после `vault_create`: показывается один раз и очищается подтверждением. */
  recoveryCode = $state<string | null>(null);

  constructor() {
    onBackendLocked(() => {
      this.lockedFromBackend();
    });
  }

  async boot(): Promise<void> {
    const status = await vaultApi.status();
    this.#setRetry(status.retryAfterMs);
    this.phase = !status.exists ? 'setup' : status.locked ? 'locked' : 'unlocked';
  }

  async create(password: string): Promise<void> {
    const { recoveryCode } = await vaultApi.create(password);
    this.recoveryCode = recoveryCode;
    this.phase = 'unlocked';
  }

  /** Пользователь подтвердил, что сохранил код: убираем его из памяти и идём дальше. */
  async acknowledgeRecovery(): Promise<void> {
    this.recoveryCode = null;
    await this.boot();
  }

  async unlock(password: string): Promise<void> {
    try {
      await vaultApi.unlock(password);
    } catch (e) {
      if (
        e instanceof ApiError &&
        (e.error.code === 'WrongPassword' || e.error.code === 'TooManyAttempts')
      ) {
        this.#setRetry(e.error.retryAfterMs);
      }
      throw e;
    }
    this.retryUntil = 0;
    this.phase = 'unlocked';
  }

  /** Перевыпуск ключа шифрования: новый recovery-код показывается тем же экраном, что при создании. */
  async rekey(password: string): Promise<void> {
    const { recoveryCode } = await vaultApi.rekey(password);
    this.recoveryCode = recoveryCode;
  }

  async unlockWithRecovery(code: string, newPassword: string): Promise<void> {
    await vaultApi.unlockRecovery(code, newPassword);
    this.retryUntil = 0;
    this.phase = 'unlocked';
  }

  async lock(): Promise<void> {
    await vaultApi.lock();
    this.lockedFromBackend();
  }

  /** Бэкенд закрыл сессию (idle, `VaultLocked`, `Locked` из команды) или это сделали мы. */
  lockedFromBackend(): void {
    for (const reset of resetters) reset();
    // Неподтверждённый recovery-код — полный доступ к данным: на заблокированном
    // приложении он оставаться на экране не должен. Новый код выдаёт перевыпуск ключа.
    this.recoveryCode = null;
    if (this.phase !== 'setup') this.phase = 'locked';
  }

  /** После `vault_reset` хранилища нет: снова первичная настройка. */
  afterReset(): void {
    for (const reset of resetters) reset();
    this.recoveryCode = null;
    this.retryUntil = 0;
    this.phase = 'setup';
  }

  #setRetry(ms: number): void {
    this.retryUntil = ms > 0 ? Date.now() + ms : 0;
  }
}

export const session = new SessionStore();

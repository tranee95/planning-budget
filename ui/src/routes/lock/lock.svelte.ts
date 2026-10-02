import { errorField, errorText } from '$lib/i18n/errors';
import { charLength, MIN_PASSWORD_CHARS } from '$lib/security/password';
import { session } from '$lib/stores/session.svelte';

export type LockView = 'recovery' | 'setup' | 'forgot' | 'unlock';

/** ViewModel экрана входа: первичная настройка, разблокировка, сброс пароля по ключу. */
export class LockVm {
  forgot = $state(false);
  busy = $state(false);
  now = $state(Date.now());

  // Первичная настройка
  setupPassword = $state('');
  setupRepeat = $state('');
  setupError = $state<string | null>(null);

  // Разблокировка
  password = $state('');
  error = $state<string | null>(null);
  shake = $state(0);

  // Сброс по ключу восстановления
  code = $state('');
  resetPassword = $state('');
  resetRepeat = $state('');
  codeError = $state<string | null>(null);
  resetPasswordError = $state<string | null>(null);

  view: LockView = $derived(
    session.recoveryCode !== null
      ? 'recovery'
      : session.phase === 'setup'
        ? 'setup'
        : this.forgot
          ? 'forgot'
          : 'unlock'
  );

  setupTooShort = $derived(
    this.setupPassword !== '' && charLength(this.setupPassword) < MIN_PASSWORD_CHARS
  );
  setupMismatch = $derived(this.setupRepeat !== '' && this.setupRepeat !== this.setupPassword);
  canSetup = $derived(
    charLength(this.setupPassword) >= MIN_PASSWORD_CHARS && this.setupRepeat === this.setupPassword
  );

  waiting = $derived(session.retryUntil > this.now);
  waitSeconds = $derived(Math.max(0, Math.ceil((session.retryUntil - this.now) / 1000)));
  canUnlock = $derived(!this.waiting && this.password !== '');

  resetMismatch = $derived(this.resetRepeat !== '' && this.resetRepeat !== this.resetPassword);
  canReset = $derived(
    this.code.trim() !== '' &&
      charLength(this.resetPassword) >= MIN_PASSWORD_CHARS &&
      this.resetRepeat === this.resetPassword
  );

  /** Обновляет `now`, пока действует пауза после неудач. Возвращает cleanup. */
  watchRetry(): () => void {
    const timer = setInterval(() => {
      this.now = Date.now();
    }, 250);
    return () => {
      clearInterval(timer);
    };
  }

  showForgot(): void {
    this.forgot = true;
  }

  backToUnlock(): void {
    this.forgot = false;
  }

  async setup(): Promise<void> {
    if (this.busy || !this.canSetup) return;
    this.busy = true;
    this.setupError = null;
    try {
      await session.create(this.setupPassword);
      this.setupPassword = '';
      this.setupRepeat = '';
    } catch (e) {
      this.setupError = errorText(e);
    } finally {
      this.busy = false;
    }
  }

  async unlock(): Promise<void> {
    if (this.busy || !this.canUnlock) return;
    this.busy = true;
    this.error = null;
    try {
      await session.unlock(this.password);
      this.password = '';
    } catch (e) {
      this.now = Date.now();
      this.shake += 1;
      this.error = errorText(e);
    } finally {
      this.busy = false;
    }
  }

  async resetByCode(): Promise<void> {
    if (this.busy || !this.canReset) return;
    this.busy = true;
    this.codeError = null;
    this.resetPasswordError = null;
    try {
      await session.unlockWithRecovery(this.code, this.resetPassword);
      this.code = '';
      this.resetPassword = '';
      this.resetRepeat = '';
    } catch (e) {
      if (errorField(e) === 'password') this.resetPasswordError = errorText(e);
      else this.codeError = errorText(e);
    } finally {
      this.busy = false;
    }
  }
}

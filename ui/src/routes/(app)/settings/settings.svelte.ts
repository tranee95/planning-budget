import type { Theme } from '$lib/api/bindings';
import { vaultApi } from '$lib/api/vault';
import { errorField, errorText } from '$lib/i18n/errors';
import { charLength, MIN_PASSWORD_CHARS } from '$lib/security/password';
import { prefs } from '$lib/stores/prefs.svelte';
import { session } from '$lib/stores/session.svelte';
import { toasts } from '$lib/stores/toasts.svelte';

export const THEMES: readonly { value: Theme; label: string }[] = [
  { value: 'light', label: 'Светлая' },
  { value: 'dark', label: 'Тёмная' },
  { value: 'system', label: 'Системная' }
];

/** ViewModel экрана «Настройки»: вид, смена пароля, перевыпуск ключа. */
export class SettingsVm {
  // Вид
  theme = $derived(prefs.current?.theme ?? 'system');
  reducedMotion = $derived(prefs.current?.reducedMotion === 'on');
  showTips = $derived(prefs.current?.showTips ?? true);
  // Switch хранит своё состояние: при ошибке сохранения View пересоздаёт его по этому ключу.
  switchKey = $state(0);

  // Смена пароля
  oldPassword = $state('');
  newPassword = $state('');
  repeat = $state('');
  oldError = $state<string | null>(null);
  newError = $state<string | null>(null);
  shake = $state(0);
  changing = $state(false);
  changed = $state(false);

  // Перевыпуск ключа
  rekeyPassword = $state('');
  rekeyError = $state<string | null>(null);
  rekeyShake = $state(0);
  rekeying = $state(false);

  mismatch = $derived(this.repeat !== '' && this.repeat !== this.newPassword);
  canChange = $derived(
    this.oldPassword !== '' &&
      charLength(this.newPassword) >= MIN_PASSWORD_CHARS &&
      this.repeat === this.newPassword
  );
  canRekey = $derived(this.rekeyPassword !== '');

  async saveView(patch: Parameters<typeof prefs.set>[0]): Promise<void> {
    try {
      await prefs.set(patch);
    } catch {
      this.switchKey++;
      toasts.push({ message: 'Не удалось сохранить настройки вида', kind: 'error' });
    }
  }

  setTheme(theme: Theme): Promise<void> {
    return this.saveView({ theme });
  }

  setReducedMotion(on: boolean): Promise<void> {
    return this.saveView({ reducedMotion: on ? 'on' : 'system' });
  }

  setShowTips(on: boolean): Promise<void> {
    return this.saveView({ showTips: on });
  }

  async changePassword(): Promise<void> {
    if (this.changing || !this.canChange) return;
    this.changing = true;
    this.changed = false;
    this.oldError = null;
    this.newError = null;
    try {
      await vaultApi.changePassword(this.oldPassword, this.newPassword);
      this.oldPassword = '';
      this.newPassword = '';
      this.repeat = '';
      this.changed = true;
    } catch (e) {
      if (errorField(e) === 'password') this.newError = errorText(e);
      else {
        this.oldError = errorText(e);
        this.shake += 1;
      }
    } finally {
      this.changing = false;
    }
  }

  async rekey(): Promise<void> {
    if (this.rekeying || !this.canRekey) return;
    this.rekeying = true;
    this.rekeyError = null;
    try {
      // При успехе стор получает новый код, и охрана маршрутов показывает экран с ним.
      await session.rekey(this.rekeyPassword);
      this.rekeyPassword = '';
    } catch (e) {
      this.rekeyError = errorText(e);
      this.rekeyShake += 1;
    } finally {
      this.rekeying = false;
    }
  }
}

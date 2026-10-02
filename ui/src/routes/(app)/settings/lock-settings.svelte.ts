import { settingsApi } from '$lib/api/data';
import { errorText } from '$lib/i18n/errors';
import { prefs } from '$lib/stores/prefs.svelte';
import { toasts } from '$lib/stores/toasts.svelte';

/** Блокировка: порог бездействия и блокировка при сворачивании окна. */
export class LockSettingsVm {
  minutes = $state('5');
  lockOnMinimize = $state(false);
  loaded = $state(false);
  error = $state<string | null>(null);
  /** Растёт при неудачном сохранении: переключатель пересоздаётся и возвращается к сохранённому. */
  switchKey = $state(0);

  async load(): Promise<void> {
    try {
      const s = await settingsApi.get();
      this.minutes = String(s.autolockMinutes);
      this.lockOnMinimize = s.lockOnMinimize;
      this.loaded = true;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async setMinutes(value: string): Promise<void> {
    const previous = this.minutes;
    this.minutes = value;
    try {
      const saved = await settingsApi.set({ autolockMinutes: Number(value) });
      this.minutes = String(saved.autolockMinutes);
      this.error = null;
      // Экран входа читает копию порога из ui-prefs.json: обновляем её в памяти.
      await prefs.load();
      toasts.push({ message: 'Автоблокировка сохранена' });
    } catch (e) {
      this.minutes = previous;
      this.error = errorText(e);
    }
  }

  async setLockOnMinimize(on: boolean): Promise<void> {
    try {
      const saved = await settingsApi.set({ lockOnMinimize: on });
      this.lockOnMinimize = saved.lockOnMinimize;
      this.error = null;
      toasts.push({ message: on ? 'Сворачивание будет блокировать' : 'Сворачивание не блокирует' });
    } catch (e) {
      this.error = errorText(e);
      this.switchKey += 1;
    }
  }
}

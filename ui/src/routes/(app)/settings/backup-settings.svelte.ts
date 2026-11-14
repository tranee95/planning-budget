import { backupApi, vaultsApi } from '$lib/api/backup';
import type { BackupStatusDto, VaultItemDto } from '$lib/api/bindings';
import { settingsApi } from '$lib/api/data';
import { errorText } from '$lib/i18n/errors';
import { toasts } from '$lib/stores/toasts.svelte';

/** Как часто создавать автоматическую копию, дни. */
export const INTERVAL_OPTIONS: readonly { value: string; label: string }[] = [
  { value: '1', label: 'Каждый день' },
  { value: '3', label: 'Раз в 3 дня' },
  { value: '7', label: 'Раз в неделю' },
  { value: '30', label: 'Раз в месяц' }
];

export function intervalOptions(current: string): readonly { value: string; label: string }[] {
  if (INTERVAL_OPTIONS.some((o) => o.value === current)) return INTERVAL_OPTIONS;
  return [{ value: current, label: `Раз в ${current} дн.` }, ...INTERVAL_OPTIONS];
}

/** Резервные копии: расписание, каталог, список и ручное создание. */
export class BackupVm {
  auto = $state(true);
  intervalDays = $state('1');
  status = $state<BackupStatusDto | null>(null);
  loaded = $state(false);
  creating = $state(false);
  error = $state<string | null>(null);
  /** Растёт при неудачном сохранении: переключатель пересоздаётся и возвращается к сохранённому. */
  switchKey = $state(0);

  async load(): Promise<void> {
    try {
      const [settings, status] = await Promise.all([settingsApi.get(), backupApi.status()]);
      this.auto = settings.backupAuto;
      this.intervalDays = String(settings.backupIntervalDays);
      this.status = status;
      this.loaded = true;
      this.error = null;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async setAuto(on: boolean): Promise<void> {
    try {
      const saved = await settingsApi.set({ backupAuto: on });
      this.auto = saved.backupAuto;
      this.error = null;
      toasts.push({ message: on ? 'Автокопии включены' : 'Автокопии выключены' });
    } catch (e) {
      this.error = errorText(e);
      this.switchKey += 1;
    }
  }

  async setInterval(value: string): Promise<void> {
    const previous = this.intervalDays;
    this.intervalDays = value;
    try {
      const saved = await settingsApi.set({ backupIntervalDays: Number(value) });
      this.intervalDays = String(saved.backupIntervalDays);
      this.error = null;
      toasts.push({ message: 'Расписание копий сохранено' });
    } catch (e) {
      this.intervalDays = previous;
      this.error = errorText(e);
    }
  }

  async create(): Promise<void> {
    if (this.creating) return;
    this.creating = true;
    try {
      await backupApi.create();
      this.status = await backupApi.status();
      this.error = null;
      toasts.push({ message: 'Копия создана' });
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.creating = false;
    }
  }

  async pickDir(): Promise<void> {
    try {
      if (await backupApi.pickDir()) {
        this.status = await backupApi.status();
        toasts.push({ message: 'Каталог копий сохранён' });
      }
      this.error = null;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async resetDir(): Promise<void> {
    try {
      await backupApi.resetDir();
      this.status = await backupApi.status();
      this.error = null;
    } catch (e) {
      this.error = errorText(e);
    }
  }
}

/** Хранилища: список, добавление, переключение, открытие копии как хранилища. */
export class VaultsVm {
  items = $state<VaultItemDto[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);

  async load(): Promise<void> {
    try {
      this.items = await vaultsApi.list();
      this.loaded = true;
      this.error = null;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** Бэкенд закрывает сессию и шлёт `VaultLocked`: окно само уходит на экран входа. */
  async run(action: () => Promise<unknown>): Promise<boolean> {
    try {
      await action();
      this.error = null;
      return true;
    } catch (e) {
      this.error = errorText(e);
      return false;
    }
  }

  switchTo(id: string): Promise<boolean> {
    return this.run(() => vaultsApi.switchTo(id));
  }

  add(label: string): Promise<boolean> {
    return this.run(() => vaultsApi.add(label));
  }

  async rename(id: string, label: string): Promise<boolean> {
    const ok = await this.run(() => vaultsApi.rename(id, label));
    if (ok) await this.load();
    return ok;
  }

  async remove(id: string): Promise<boolean> {
    const ok = await this.run(() => vaultsApi.remove(id));
    if (ok) await this.load();
    return ok;
  }
}

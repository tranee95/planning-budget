import type { UpdateStatusDto } from '$lib/api/bindings';
import { events } from '$lib/api/bindings';
import { updatesApi } from '$lib/api/updates';
import { toasts } from './toasts.svelte';

/** Состояние проверки обновлений: итог, ход установки и «Позже» для окна. */
class UpdatesStore {
  status = $state.raw<UpdateStatusDto | null>(null);
  checking = $state(false);
  installing = $state(false);
  /** Процент загрузки; `null` — размер неизвестен или загрузка не начата. */
  progress = $state<number | null>(null);
  /** Окно «Доступна версия» закрыто кнопкой «Позже» до следующего запуска. */
  dismissed = $state(false);
  #autoDone = false;

  /** Есть новая версия, о которой стоит спросить. */
  get offer(): UpdateStatusDto | null {
    return this.status?.state === 'available' && !this.dismissed ? this.status : null;
  }

  /** Автопроверка при запуске: один раз за запуск, после разблокировки. Сбой не показывается. */
  async autoCheck(): Promise<void> {
    if (this.#autoDone) return;
    this.#autoDone = true;
    try {
      const status = await updatesApi.check(false);
      // «Пропущено» и «выключено» не затирают прежний итог ручной проверки.
      if (status.state !== 'skipped' && status.state !== 'disabled') this.status = status;
    } catch {
      // Проверка обновлений не должна мешать работе.
    }
  }

  /** Кнопка «Проверить сейчас». */
  async checkNow(): Promise<void> {
    if (this.checking || this.installing) return;
    this.checking = true;
    try {
      this.status = await updatesApi.check(true);
      this.dismissed = false;
    } catch (e) {
      toasts.fail(e);
    } finally {
      this.checking = false;
    }
  }

  dismiss(): void {
    this.dismissed = true;
  }

  /** Скачивает и ставит. При успехе приложение перезапускается и вызов не возвращается. */
  async install(): Promise<void> {
    if (this.installing) return;
    this.installing = true;
    this.progress = null;
    let off: (() => void) | undefined;
    try {
      off = await events.updateProgress.listen((event) => {
        this.progress = event.payload.percent;
      });
    } catch {
      // Без прогресса установка идёт как обычно.
    }
    try {
      await updatesApi.install();
    } catch (e) {
      toasts.fail(e);
    } finally {
      off?.();
      this.installing = false;
      this.progress = null;
    }
  }

  reset(): void {
    this.status = null;
    this.checking = false;
    this.installing = false;
    this.progress = null;
    this.dismissed = false;
    this.#autoDone = false;
  }
}

export const updates = new UpdatesStore();

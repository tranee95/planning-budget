import { settingsApi } from '$lib/api/data';
import { parsePercentBp } from '$lib/category-colors';
import { errorText } from '$lib/i18n/errors';
import { toasts } from '$lib/stores/toasts.svelte';

const toText = (bp: number): string => String(bp / 100).replace('.', ',');

/** Коридор сбережений: нижняя граница, норма и верхняя граница, проценты от дохода. */
export class SavingsVm {
  min = $state('');
  norm = $state('');
  max = $state('');
  error = $state<string | null>(null);
  saving = $state(false);
  loaded = $state(false);

  async load(): Promise<void> {
    try {
      const s = await settingsApi.get();
      this.min = toText(s.savingsMinBp);
      this.norm = toText(s.savingsNormBp);
      this.max = toText(s.savingsMaxBp);
      this.loaded = true;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async save(): Promise<void> {
    const min = parsePercentBp(this.min);
    const norm = parsePercentBp(this.norm);
    const max = parsePercentBp(this.max);
    if (min === null || norm === null || max === null) {
      this.error = 'Проценты от 0 до 100, например 13 или 14,5';
      return;
    }
    if (!(min <= norm && norm <= max)) {
      this.error = 'Нижняя граница не больше нормы, норма не больше верхней границы';
      return;
    }
    this.error = null;
    this.saving = true;
    try {
      await settingsApi.set({ savingsMinBp: min, savingsNormBp: norm, savingsMaxBp: max });
      toasts.push({ message: 'Коридор сбережений сохранён' });
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.saving = false;
    }
  }
}

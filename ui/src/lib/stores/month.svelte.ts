import { currentMonth, shiftMonth } from '$lib/format';

/** Глобальный выбранный месяц `YYYY-MM`: сохраняется при переходах между разделами. */
export class MonthStore {
  current = $state('');
  /** Куда сдвинулся месяц последний раз: контент анимируется в эту сторону. */
  direction = $state<1 | -1>(1);
  #now: () => string;

  constructor(initial: string = currentMonth(), now: () => string = currentMonth) {
    this.current = initial;
    this.#now = now;
  }

  set(month: string): void {
    if (month === this.current) return;
    this.direction = month > this.current ? 1 : -1;
    this.current = month;
  }

  shift(delta: number): void {
    this.set(shiftMonth(this.current, delta));
  }

  today(): void {
    this.set(this.#now());
  }
}

export const month = new MonthStore();

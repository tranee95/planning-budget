/** Верх шкалы значений: ближайшее «круглое» число над максимумом с небольшим запасом, а не вдвое выше. */

/** Столбец максимума не должен упираться в верх графика. */
const HEADROOM = 1.05;
const STEPS = [1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10];

/**
 * Верх оси из `ticks` равных делений: шаг — круглое число (1, 1.5, 2, 2.5, 3, 4, 5, 6, 8 × 10ⁿ),
 * такой, что верх не ниже `peak`. Для `peak <= 0` возвращает 0.
 */
export function niceTop(peak: number, ticks: number): number {
  if (!(peak > 0) || ticks < 1) return 0;
  const magnitude = 10 ** Math.floor(Math.log10(peak / ticks));
  const target = peak * HEADROOM;
  const step = STEPS.find((k) => k * magnitude * ticks >= target) ?? 10;
  return step * magnitude * ticks;
}

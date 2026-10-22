import type { CategoryKindDto } from '$lib/api/bindings';

/** Превышение плана — это хорошо: отложить или погасить больше запланированного. У трат это перерасход. */
export function overIsGood(kind: CategoryKindDto): boolean {
  return kind === 'savings' || kind === 'loans';
}

export type DeviationTone = 'bad' | 'good' | 'none';

/** Цвет отклонения «факт − план»: смысл зависит от типа статьи. */
export function deviationTone(kind: CategoryKindDto, deviation: number): DeviationTone {
  if (deviation <= 0) return 'none';
  return overIsGood(kind) ? 'good' : 'bad';
}

import { POPULAR_PASSWORDS } from './popular-passwords';

/** Минимальная длина в символах; совпадает с `MIN_PASSWORD_CHARS` в crates/vault. */
export const MIN_PASSWORD_CHARS = 10;

export type Strength = {
  /** 0 — слабый … 3 — надёжный */
  score: 0 | 1 | 2 | 3;
  label: 'Слабый' | 'Средний' | 'Хороший' | 'Надёжный';
  popular: boolean;
};

const MIN_DISTINCT_CHARS = 5;
const LABELS = ['Слабый', 'Средний', 'Хороший', 'Надёжный'] as const;

/** Длина в символах (кодовых точках), как в Rust `chars().count()`. */
export function charLength(password: string): number {
  return Array.from(password).length;
}

function classCount(password: string): number {
  const classes = [/\p{Ll}/u, /\p{Lu}/u, /\p{Nd}/u, /[^\p{L}\p{Nd}]/u];
  return classes.filter((re) => re.test(password)).length;
}

/**
 * Простая оценка по длине и классам символов, без внешних словарей.
 * Пароль из списка популярных всегда «Слабый».
 */
export function passwordStrength(password: string): Strength {
  const popular = POPULAR_PASSWORDS.has(password.toLowerCase());
  const length = charLength(password);
  let score: 0 | 1 | 2 | 3 = 0;
  // Повторы вроде «aaaaaaaaaaaaaa» длинные, но угадываются сразу: меньше 5 разных символов — слабый.
  const varied = new Set(Array.from(password)).size >= MIN_DISTINCT_CHARS;
  if (!popular && varied && length >= MIN_PASSWORD_CHARS) {
    const classes = classCount(password);
    if (length >= 16 && classes >= 3) score = 3;
    else if (length >= 14 || (length >= 12 && classes >= 3)) score = 2;
    else score = classes >= 2 ? 1 : 0;
  }
  return { score, label: LABELS[score], popular };
}

import type { QueryHintDto, TokenKindDto } from '$lib/api/bindings';

/** Позиции токенов приходят в символах (Rust `char`), а не в UTF-16: режем по кодовым точкам. */
export function sliceChars(text: string, start: number, end: number): string {
  return Array.from(text).slice(start, end).join('');
}

/** Запрос без токена `start..end` и лишних пробелов вокруг него. */
export function removeSpan(text: string, start: number, end: number): string {
  const chars = Array.from(text);
  return [...chars.slice(0, start), ...chars.slice(end)]
    .join('')
    .replace(/\s{2,}/g, ' ')
    .trim();
}

export const HINT_TEXT: Record<QueryHintDto, string> = {
  empty_value: 'не указано значение после «:»',
  unknown_value: 'такого значения нет, токен ищется как текст',
  bad_amount: 'сумма: «>5000», «<=1000» или «1000..5000»',
  bad_month: 'период: «сен», «2026-09», «июн..сен» или «2026»',
  negation_unsupported: 'минус работает только с кат: и #тегом'
};

export const KIND_LABEL: Record<TokenKindDto, string> = {
  text: 'текст',
  amount: 'сумма',
  status: 'статус',
  category: 'категория',
  kind: 'тип',
  month: 'период',
  tag: 'тег',
  source: 'источник',
  record: 'вид'
};

const KEYS = ['статус', 'категория', 'тип', 'период', 'сумма', 'источник'] as const;

const VALUES: Record<string, readonly string[]> = {
  статус: ['оплачено', 'долг', 'внеплан', 'план'],
  тип: ['обязательные', 'желания', 'сбережения', 'займы'],
  источник: ['вручную', 'импорт']
};

const norm = (s: string): string => s.toLowerCase().replaceAll('ё', 'е');

/**
 * Дополнение по Tab: недописанный ключ → `ключ:`, недописанное значение → полное слово.
 * `null`, если последнее слово нечем дополнить (тогда Tab остаётся обычным переходом фокуса).
 */
export function completeToken(query: string): string | null {
  const at = query.search(/\S+$/);
  if (at < 0) return null;
  const word = query.slice(at);
  const head = query.slice(0, at);
  const negation = word.startsWith('-') ? '-' : '';
  const body = norm(word.slice(negation.length));
  if (body === '' || body.startsWith('"') || body.startsWith('#')) return null;

  const colon = body.indexOf(':');
  if (colon < 0) {
    const key = KEYS.find((k) => k.startsWith(body));
    return key ? `${head}${negation}${key}:` : null;
  }
  const values = VALUES[body.slice(0, colon)];
  const typed = body.slice(colon + 1);
  const value = values?.find((v) => typed !== '' && v.startsWith(typed) && v !== typed);
  return value ? `${head}${negation}${body.slice(0, colon + 1)}${value}` : null;
}

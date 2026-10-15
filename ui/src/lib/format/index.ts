const NBSP = ' ';
const MINUS = '−';

const MONTHS = [
  'Январь',
  'Февраль',
  'Март',
  'Апрель',
  'Май',
  'Июнь',
  'Июль',
  'Август',
  'Сентябрь',
  'Октябрь',
  'Ноябрь',
  'Декабрь'
] as const;

const MONTHS_PREPOSITIONAL = [
  'январе',
  'феврале',
  'марте',
  'апреле',
  'мае',
  'июне',
  'июле',
  'августе',
  'сентябре',
  'октябре',
  'ноябре',
  'декабре'
] as const;

// Intl.NumberFormat дорогой в создании: форматтеры живут на уровне модуля.
const grouped = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 0, useGrouping: true });
const exact = new Intl.NumberFormat('ru-RU', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2
});
const fraction = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 2 });

// В ru-RU разделитель тысяч — U+00A0 или U+202F в зависимости от ICU; приводим к одному.
const normalize = (s: string): string => s.replace(/\s/g, NBSP);

/** Рубли без копеек: копейки округляются от нуля. Вход — копейки (`Money`). */
export function formatMoney(kopecks: number): string {
  const abs = Math.abs(kopecks);
  const rubles = Math.floor((abs + 50) / 100);
  const sign = kopecks < 0 && rubles > 0 ? MINUS : '';
  return `${sign}${normalize(grouped.format(rubles))}${NBSP}₽`;
}

/** Рубли с копейками — для полей редактирования и точных сверок. */
export function formatMoneyExact(kopecks: number): string {
  const abs = Math.abs(kopecks);
  const sign = kopecks < 0 ? MINUS : '';
  return `${sign}${normalize(exact.format(abs / 100))}${NBSP}₽`;
}

/** Рубли; копейки показываются только если они есть: «20 000 ₽», «20 000,50 ₽». */
export function formatMoneyTrim(kopecks: number): string {
  return kopecks % 100 === 0 ? formatMoney(kopecks) : formatMoneyExact(kopecks);
}

/** Базисные пункты → проценты: 1350 → «13,5 %». */
export function formatPercent(basisPoints: number): string {
  const sign = basisPoints < 0 ? MINUS : '';
  return `${sign}${normalize(fraction.format(Math.abs(basisPoints) / 100))}${NBSP}%`;
}

const weeks = new Intl.NumberFormat('ru-RU', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1
});

/** Недели одним знаком после запятой: 16/7 → «2,3». Само число приходит из ядра. */
export function formatWeeks(value: number): string {
  return weeks.format(value);
}

/** `YYYY-MM` → «Сентябрь 2026» (`short` — без года). */
export function formatMonth(month: string, style: 'long' | 'short' = 'long'): string {
  const [year, number] = parseMonth(month);
  const name = MONTHS[number - 1] ?? month;
  return style === 'short' ? name : `${name} ${String(year)}`;
}

/** Форма слова по числу: `forms` = [для 1, для 2–4, для 5–20] («категория», «категории», «категорий»). */
export function pluralRu(n: number, forms: readonly [string, string, string]): string {
  const mod10 = Math.abs(n) % 10;
  const mod100 = Math.abs(n) % 100;
  if (mod10 === 1 && mod100 !== 11) return forms[0];
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return forms[1];
  return forms[2];
}

/** `YYYY-MM` → «сентябре 2026»: для фраз «в сентябре 2026». */
export function formatMonthPrepositional(month: string): string {
  const [year, number] = parseMonth(month);
  const name = MONTHS_PREPOSITIONAL[number - 1];
  return name === undefined ? month : `${name} ${String(year)}`;
}

/** Сдвигает `YYYY-MM` на `delta` месяцев. */
export function shiftMonth(month: string, delta: number): string {
  const [year, number] = parseMonth(month);
  const index = year * 12 + (number - 1) + delta;
  const y = Math.floor(index / 12);
  const m = (index % 12) + 1;
  return `${String(y)}-${String(m).padStart(2, '0')}`;
}

/** Текущий месяц по локальным часам в формате `YYYY-MM`. */
export function currentMonth(now: Date = new Date()): string {
  return `${String(now.getFullYear())}-${String(now.getMonth() + 1).padStart(2, '0')}`;
}

/** Сегодняшняя дата `YYYY-MM-DD` в часовом поясе пользователя. */
export function today(now: Date = new Date()): string {
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${String(now.getFullYear())}-${month}-${day}`;
}

/** Первый и последний день месяца `YYYY-MM` в формате `YYYY-MM-DD`. */
export function monthRange(month: string): { first: string; last: string } {
  const [year, number] = parseMonth(month);
  const days = new Date(year, number, 0).getDate();
  return { first: `${month}-01`, last: `${month}-${String(days).padStart(2, '0')}` };
}

function parseMonth(month: string): [number, number] {
  const [y, m] = month.split('-');
  return [Number(y), Number(m)];
}

/** Ввод пользователя → копейки. Допускает «2596,5», «2 596.50», «1000 ₽»; иначе `null`. */
export function parseMoney(text: string): number | null {
  const result = checkMoney(text);
  return result.ok ? result.kopecks : null;
}

/** Наибольшая сумма в поле ввода: 999 999 999,99 ₽. */
export const MAX_KOPECKS = 99_999_999_999;

export type MoneyCheck = { ok: true; kopecks: number } | { ok: false; reason: 'format' | 'range' };

/** Разбор ввода с причиной отказа: неверный формат или сумма больше `MAX_KOPECKS`. */
export function checkMoney(text: string): MoneyCheck {
  const cleaned = text.replace(/[\s₽]/g, '').replace(',', '.');
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(cleaned);
  if (!match) return { ok: false, reason: 'format' };
  const rubles = Number(match[1]);
  const kopecks = Number((match[2] ?? '').padEnd(2, '0'));
  const total = rubles * 100 + kopecks;
  return total > MAX_KOPECKS ? { ok: false, reason: 'range' } : { ok: true, kopecks: total };
}

/** `YYYY-MM-DD` → «05.09»; траты без даты показываются прочерком. */
export function formatDay(date: string | null): string {
  if (date === null) return '—';
  const [, month = '', day = ''] = date.split('-');
  return `${day}.${month}`;
}

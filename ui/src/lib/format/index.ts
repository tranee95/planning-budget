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

const oneDecimal = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 1 });

/** Проценты для сводок и KPI: целые («19 %»), без ложной точности. */
export function formatPercentWhole(basisPoints: number): string {
  const whole = Math.round(Math.abs(basisPoints) / 100);
  // знак минус только у ненулевого результата: «−0 %» не бывает
  const sign = basisPoints < 0 && whole !== 0 ? MINUS : '';
  return `${sign}${String(whole)}${NBSP}%`;
}

/** Проценты на графиках: не больше одного знака («12,3 %», «14 %»). */
export function formatPercentShort(basisPoints: number): string {
  const shown = oneDecimal.format(Math.abs(basisPoints) / 100);
  // минус только у ненулевого результата: «−0 %» не бывает
  const sign = basisPoints < 0 && Math.round(Math.abs(basisPoints) / 10) !== 0 ? MINUS : '';
  return `${sign}${normalize(shown)}${NBSP}%`;
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

const MONTHS_GENITIVE = [
  'января',
  'февраля',
  'марта',
  'апреля',
  'мая',
  'июня',
  'июля',
  'августа',
  'сентября',
  'октября',
  'ноября',
  'декабря'
] as const;

/** `YYYY-MM` → «октября 2026»: для фраз «действует с октября 2026». */
export function formatMonthGenitive(month: string): string {
  const [year, number] = parseMonth(month);
  const name = MONTHS_GENITIVE[number - 1];
  return name === undefined ? month : `${name} ${String(year)}`;
}

/** `YYYY-MM` → «октябрь 2026» с маленькой буквы: для фраз «план на октябрь 2026», «взято · март 2026». */
export function formatMonthLower(month: string): string {
  return formatMonth(month).toLowerCase();
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

/** Текст поля суммы: «30 000», копейки только если они есть — «30 000,50»; без знака ₽. */
export function formatMoneyField(kopecks: number): string {
  const text = formatMoneyTrim(Math.abs(kopecks)).replace(/\s?₽$/, '');
  return text.replace(/^−/, '');
}

/**
 * Разряды на лету: «15000» → «15 000», «1234,5» → «1 234,5». Точка и запятая — разделитель копеек,
 * знаки после него остаются как набраны (лишние покажет проверка поля). Текст с буквами и прочим не меняется: поле само покажет ошибку.
 */
export function groupMoneyInput(text: string): string {
  if (!/^[\d\s.,]*$/.test(text)) return text;
  const compact = text.replace(/\s/g, '');
  const cut = compact.search(/[.,]/);
  const whole = cut === -1 ? compact : compact.slice(0, cut);
  const fractionPart = cut === -1 ? null : compact.slice(cut + 1).replace(/[.,]/g, '');
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, NBSP);
  return fractionPart === null ? grouped : `${grouped},${fractionPart}`;
}

/** `2026-09-05` → «05.09.2026»; пустое значение — пустая строка. */
export function isoToDateField(iso: string | null | undefined): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso ?? '');
  return match ? `${match[3] ?? ''}.${match[2] ?? ''}.${match[1] ?? ''}` : '';
}

/** «05.09.2026» → `2026-09-05`; несуществующая дата (31.02) и неполный ввод дают `null`. */
export function dateFieldToIso(text: string): string | null {
  const match = /^(\d{2})\.(\d{2})\.(\d{4})$/.exec(text);
  if (!match) return null;
  const [day, month, year] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const date = new Date(Date.UTC(year, month - 1, day));
  if (
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() !== month - 1 ||
    date.getUTCDate() !== day
  )
    return null;
  return `${match[3] ?? ''}-${match[2] ?? ''}-${match[1] ?? ''}`;
}

/** Маска «дд.мм.гггг» на лету: только цифры, точки ставятся сами, не длиннее 10 знаков. */
export function maskDateInput(text: string): string {
  const digits = text.replace(/\D/g, '').slice(0, 8);
  const parts = [digits.slice(0, 2), digits.slice(2, 4), digits.slice(4)].filter((p) => p !== '');
  const masked = parts.join('.');
  // «05» — день закончен, точка нужна сразу, чтобы курсор не стоял перед ней
  return digits.length === 2 || digits.length === 4 ? `${masked}.` : masked;
}

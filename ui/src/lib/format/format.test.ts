import { describe, expect, it } from 'vitest';
import {
  checkMoney,
  formatDay,
  formatMonth,
  formatMoney,
  formatMoneyExact,
  formatPercent,
  monthRange,
  parseMoney,
  shiftMonth
} from './index';

const NB = ' ';
const MINUS = '−';

describe('formatMoney', () => {
  it('округляет копейки до рублей и разделяет тысячи неразрывным пробелом', () => {
    expect(formatMoney(123_456_00)).toBe(`123${NB}456${NB}₽`);
    expect(formatMoney(2596_50)).toBe(`2${NB}597${NB}₽`);
    expect(formatMoney(0)).toBe(`0${NB}₽`);
  });

  it('округляет половину рубля от нуля, в том числе для отрицательных', () => {
    expect(formatMoney(49)).toBe(`0${NB}₽`);
    expect(formatMoney(50)).toBe(`1${NB}₽`);
    expect(formatMoney(-50)).toBe(`${MINUS}1${NB}₽`);
  });

  it('пишет минус как U+2212 и не показывает «−0»', () => {
    expect(formatMoney(-1_012_00)).toBe(`${MINUS}1${NB}012${NB}₽`);
    expect(formatMoney(-10)).toBe(`0${NB}₽`);
  });
});

describe('formatMoneyExact', () => {
  it('всегда показывает копейки', () => {
    expect(formatMoneyExact(2596_50)).toBe(`2${NB}596,50${NB}₽`);
    expect(formatMoneyExact(100_00)).toBe(`100,00${NB}₽`);
    expect(formatMoneyExact(-5)).toBe(`${MINUS}0,05${NB}₽`);
  });
});

describe('formatPercent', () => {
  it('принимает базисные пункты', () => {
    expect(formatPercent(1300)).toBe(`13${NB}%`);
    expect(formatPercent(1350)).toBe(`13,5${NB}%`);
    expect(formatPercent(-800)).toBe(`${MINUS}8${NB}%`);
  });
});

describe('formatMonth', () => {
  it('принимает YYYY-MM и пишет русское название', () => {
    expect(formatMonth('2026-09')).toBe('Сентябрь 2026');
    expect(formatMonth('2026-09', 'short')).toBe('Сентябрь');
  });
});

describe('shiftMonth', () => {
  it('переходит через границу года', () => {
    expect(shiftMonth('2026-12', 1)).toBe('2027-01');
    expect(shiftMonth('2026-01', -1)).toBe('2025-12');
    expect(shiftMonth('2026-05', 0)).toBe('2026-05');
  });
});

describe('parseMoney', () => {
  it('разбирает рубли с запятой или точкой и пробелами', () => {
    expect(parseMoney('2596,5')).toBe(2596_50);
    expect(parseMoney('2 596.50')).toBe(2596_50);
    expect(parseMoney(`1${NB}000 ₽`)).toBe(1000_00);
    expect(parseMoney('7')).toBe(700);
  });

  it('возвращает null для пустого и некорректного ввода', () => {
    expect(parseMoney('')).toBeNull();
    expect(parseMoney('abc')).toBeNull();
    expect(parseMoney('1,234')).toBeNull();
    expect(parseMoney('1,2,3')).toBeNull();
  });
});

describe('верхняя граница суммы', () => {
  it('принимает 999 999 999,99 ₽ и отклоняет всё, что больше', () => {
    expect(parseMoney('999999999,99')).toBe(99_999_999_999);
    expect(parseMoney('1000000000')).toBeNull();
    expect(parseMoney('999999999999')).toBeNull();
  });

  it('checkMoney различает неверный формат и слишком большую сумму', () => {
    expect(checkMoney('12 345,00 ₽')).toEqual({ ok: true, kopecks: 12_345_00 });
    expect(checkMoney('abc')).toEqual({ ok: false, reason: 'format' });
    expect(checkMoney('-50')).toEqual({ ok: false, reason: 'format' });
    expect(checkMoney('1000000000')).toEqual({ ok: false, reason: 'range' });
  });
});

describe('formatDay', () => {
  it('YYYY-MM-DD → «дд.мм», отсутствие даты — прочерк', () => {
    expect(formatDay('2026-09-05')).toBe('05.09');
    expect(formatDay(null)).toBe('—');
  });
});

describe('monthRange', () => {
  it('первый и последний день месяца, включая февраль и високосный год', () => {
    expect(monthRange('2026-09')).toEqual({ first: '2026-09-01', last: '2026-09-30' });
    expect(monthRange('2026-12').last).toBe('2026-12-31');
    expect(monthRange('2026-02').last).toBe('2026-02-28');
    expect(monthRange('2028-02').last).toBe('2028-02-29');
  });
});

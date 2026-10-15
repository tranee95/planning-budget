import { describe, expect, it } from 'vitest';
import {
  checkMoney,
  formatDay,
  formatMonth,
  formatMonthPrepositional,
  formatMoney,
  formatMoneyExact,
  formatMoneyTrim,
  formatPercent,
  formatWeeks,
  monthRange,
  parseMoney,
  pluralRu,
  shiftMonth
} from './index';

describe('pluralRu', () => {
  const open = ['открытый', 'открытых', 'открытых'] as const;
  it.each([
    [0, 'открытых'],
    [1, 'открытый'],
    [2, 'открытых'],
    [5, 'открытых'],
    [11, 'открытых'],
    [21, 'открытый'],
    [22, 'открытых'],
    [101, 'открытый']
  ])('%i → %s', (n, word) => {
    expect(pluralRu(n, open)).toBe(word);
  });

  it('три разные формы: категория, категории, категорий', () => {
    const forms = ['категория', 'категории', 'категорий'] as const;
    expect([1, 2, 4, 5, 11, 12, 14, 21, 25].map((n) => pluralRu(n, forms))).toEqual([
      'категория',
      'категории',
      'категории',
      'категорий',
      'категорий',
      'категорий',
      'категорий',
      'категория',
      'категорий'
    ]);
  });
});

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

describe('formatMonthPrepositional', () => {
  it('предложный падеж всех двенадцати месяцев с годом', () => {
    const expected = [
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
    ];
    expected.forEach((name, i) => {
      const month = `2026-${String(i + 1).padStart(2, '0')}`;
      expect(formatMonthPrepositional(month)).toBe(`${name} 2026`);
    });
  });

  it('formatMonth остаётся в именительном падеже', () => {
    expect(formatMonth('2026-09')).toBe('Сентябрь 2026');
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

describe('formatWeeks', () => {
  it('недели с одним знаком после запятой и запятой как разделителем', () => {
    expect(formatWeeks(16 / 7)).toBe('2,3');
    expect(formatWeeks(1 / 7)).toBe('0,1');
    expect(formatWeeks(30 / 7)).toBe('4,3');
    expect(formatWeeks(2)).toBe('2,0');
  });
});

describe('formatMoneyTrim', () => {
  it('копейки показываются только если они есть', () => {
    expect(formatMoneyTrim(2_000_000)).toBe('20 000 ₽');
    expect(formatMoneyTrim(2_000_050)).toBe('20 000,50 ₽');
    expect(formatMoneyTrim(0)).toBe('0 ₽');
  });
});

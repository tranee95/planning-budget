import { describe, expect, test } from 'vitest';
import { completeToken, removeSpan, sliceChars } from './query';

describe('позиции токенов', () => {
  test('режут по символам, а не по UTF-16', () => {
    expect(sliceChars('😀 лента статус:план', 2, 7)).toBe('лента');
  });

  test('удаление токена схлопывает пробелы', () => {
    expect(removeSpan('лента статус:оплачено сумма>5000', 6, 21)).toBe('лента сумма>5000');
    expect(removeSpan('лента', 0, 5)).toBe('');
  });
});

describe('completeToken', () => {
  test.each([
    ['лента стат', 'лента статус:'],
    ['кат', 'категория:'],
    ['статус', 'статус:'],
    ['-кат', '-категория:'],
    ['статус:опл', 'статус:оплачено'],
    ['лента тип:жел', 'лента тип:желания'],
    ['Статус:ВНЕ', 'статус:внеплан']
  ])('«%s» → «%s»', (input, expected) => {
    expect(completeToken(input)).toBe(expected);
  });

  test.each([['лента'], [''], ['лента '], ['статус:оплачено'], ['#под'], ['"стат'], ['сумма>5']])(
    '«%s» нечем дополнить',
    (input) => {
      expect(completeToken(input)).toBeNull();
    }
  );
});

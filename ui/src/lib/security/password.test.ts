import { expect, test } from 'vitest';
import { charLength, MIN_PASSWORD_CHARS, passwordStrength } from './password';

test('минимальная длина совпадает с бэкендом', () => {
  expect(MIN_PASSWORD_CHARS).toBe(10);
});

test('длина считается в символах, как chars().count() в Rust', () => {
  expect(charLength('пароль')).toBe(6);
  expect(charLength('😀😀')).toBe(2);
  expect(charLength('')).toBe(0);
});

test.each([
  ['', 0, 'Слабый'],
  ['короткий', 0, 'Слабый'],
  ['aaaaaaaaaa', 0, 'Слабый'],
  ['aaaaaaaaaaaaaaaa', 0, 'Слабый'],
  ['abababababababab', 0, 'Слабый'],
  ['abcdefghij1', 1, 'Средний'],
  ['abcdefghij12345', 2, 'Хороший'],
  ['Correct-Horse-Battery9', 3, 'Надёжный']
] as const)('надёжность «%s» → %i (%s)', (password, score, label) => {
  const s = passwordStrength(password);
  expect(s.score).toBe(score);
  expect(s.label).toBe(label);
});

test('популярный пароль помечается и всегда слабый, регистр не важен', () => {
  const s = passwordStrength('Qwerty1234');
  expect(s.popular).toBe(true);
  expect(s.score).toBe(0);
  expect(passwordStrength('Correct-Horse-Battery9').popular).toBe(false);
});

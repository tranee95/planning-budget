import { expect, it } from 'vitest';
import { NARROW_PLACEHOLDERS, PLACEHOLDERS } from './placeholders';

const entries = Object.entries(PLACEHOLDERS) as [keyof typeof PLACEHOLDERS, string][];

it('подсказки не пустые, без лишних пробелов и без двоеточия на конце', () => {
  for (const [key, text] of entries) {
    expect(text.trim(), key).toBe(text);
    expect(text.length, key).toBeGreaterThan(0);
    expect(text.endsWith(':'), key).toBe(false);
  }
});

it('подсказки укладываются в длину: 56 символов, в узких полях — 18', () => {
  for (const [key, text] of entries) expect(text.length, key).toBeLessThanOrEqual(56);
  for (const key of NARROW_PLACEHOLDERS)
    expect(PLACEHOLDERS[key].length, key).toBeLessThanOrEqual(18);
});

it('подсказки-примеры начинаются со слова «Например», числа в примерах с пробелом тысяч', () => {
  for (const [key, text] of entries) {
    if (!text.startsWith('Например')) continue;
    expect(text, key).toMatch(/^Например, \S/);
    const amount = text.match(/\d[\d\s]*\d/)?.[0];
    if (amount && amount.replace(/\s/g, '').length >= 4) expect(amount, key).toContain(' ');
  }
});

it('одинаковых подсказок у разных полей нет', () => {
  const texts = entries.map(([, text]) => text);
  expect(new Set(texts).size).toBe(texts.length);
});

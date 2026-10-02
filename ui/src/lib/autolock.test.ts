import { describe, expect, it } from 'vitest';
import { autolockLine } from './autolock';

describe('autolockLine', () => {
  it.each([
    [1, 'Автоблокировка через 1 минуту бездействия'],
    [5, 'Автоблокировка через 5 минут бездействия'],
    [15, 'Автоблокировка через 15 минут бездействия'],
    [2, 'Автоблокировка через 2 минуты бездействия'],
    [0, 'Автоблокировка отключена']
  ])('%i мин → «%s»', (minutes, text) => {
    expect(autolockLine(minutes)).toBe(text);
  });
});

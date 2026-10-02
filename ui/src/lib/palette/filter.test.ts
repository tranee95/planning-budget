import { describe, expect, it } from 'vitest';
import { themeCommands } from './commands';
import { filterCommands, type PaletteCommand } from './filter';

const cmd = (id: string, label: string, keywords = ''): PaletteCommand => ({
  id,
  label,
  group: 'Переход',
  keywords,
  run: () => {}
});

const items = [
  cmd('overview', 'Обзор'),
  cmd('settings', 'Настройки'),
  cmd('theme-dark', 'Тема: тёмная', 'dark')
];

describe('filterCommands', () => {
  it('без запроса возвращает всё в исходном порядке', () => {
    expect(filterCommands(items, '').map((c) => c.id)).toEqual([
      'overview',
      'settings',
      'theme-dark'
    ]);
  });

  it('ищет по подстроке без учёта регистра', () => {
    expect(filterCommands(items, 'НАСТ').map((c) => c.id)).toEqual(['settings']);
  });

  it('считает ё и е одной буквой', () => {
    expect(filterCommands(items, 'темная').map((c) => c.id)).toEqual(['theme-dark']);
  });

  it('учитывает ключевые слова', () => {
    expect(filterCommands(items, 'dark').map((c) => c.id)).toEqual(['theme-dark']);
  });

  it('начало названия идёт раньше вхождения в середине', () => {
    const list = [cmd('a', 'Новая тема'), cmd('b', 'Тема: светлая')];
    expect(filterCommands(list, 'тема').map((c) => c.id)).toEqual(['b', 'a']);
  });
});

describe('команды тем', () => {
  const themes = themeCommands(() => {});
  const ids = (q: string): string[] => filterCommands(themes, q).map((c) => c.id);

  it.each([
    ['тем', ['theme-light', 'theme-dark', 'theme-system']],
    ['темная', ['theme-dark', 'theme-system']],
    ['тёмная', ['theme-dark', 'theme-system']],
    ['ТЁМН', ['theme-dark', 'theme-system']],
    ['светл', ['theme-light']],
    ['dark', ['theme-dark']],
    ['LIGHT', ['theme-light']],
    ['как в', ['theme-system']],
    ['авто', ['theme-system']]
  ])('запрос «%s» находит нужные темы', (query, expected) => {
    expect(ids(query)).toEqual(expect.arrayContaining(expected));
  });

  it('«тёмная» стоит выше «системной», хотя та тоже содержит «темная»', () => {
    expect(ids('тёмная')[0]).toBe('theme-dark');
  });

  it('склеенный из двух запросов текст ничего не находит', () => {
    expect(ids('темнаясветлая')).toEqual([]);
  });
});

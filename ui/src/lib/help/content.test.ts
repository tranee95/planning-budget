import { expect, it } from 'vitest';
import { HELP_SECTIONS, INTRO_STEPS, KEY_GROUPS, STATUS_EXPLAINED } from './content';

it('справка: «Как спланировать месяц» после цикла и выше «Данные остаются у вас»', () => {
  const ids = HELP_SECTIONS.map((s) => s.id);
  expect(ids).toEqual([
    'idea',
    'cycle',
    'first-month',
    'statuses',
    'savings-debts',
    'privacy',
    'search'
  ]);
});

it('шаг «Сбережения и долги» в знакомстве короче, чем в справке, и умещается в три строки', () => {
  const intro = INTRO_STEPS.find((s) => s.id === 'savings-debts');
  const help = HELP_SECTIONS.find((s) => s.id === 'savings-debts');
  const length = (s?: { paragraphs: readonly string[] }): number =>
    (s?.paragraphs ?? []).join(' ').length;
  expect(intro?.paragraphs.length).toBeLessThanOrEqual(2);
  expect(length(intro)).toBeLessThan(330);
  expect(length(help)).toBeGreaterThan(length(intro));
});

it('тексты не обещают «нет сети»: проверка обновлений названа, место отключения указано', () => {
  const all = [...INTRO_STEPS, ...HELP_SECTIONS].flatMap((s) => s.paragraphs).join('\n');
  expect(all).toContain('кроме проверки обновлений');
  expect(all).toContain('«Настройки → Вид»');
  expect(all).not.toMatch(/не обращается к сети\./);
});

it('идентификаторы разделов уникальны, у каждого есть заголовок и текст', () => {
  const all = HELP_SECTIONS;
  expect(new Set(all.map((s) => s.id)).size).toBe(all.length);
  for (const section of all) {
    expect(section.title.trim()).not.toBe('');
    expect(section.paragraphs.length).toBeGreaterThan(0);
  }
});

it('знакомство — пять шагов, как', () => {
  expect(INTRO_STEPS.map((s) => s.id)).toEqual([
    'idea',
    'cycle',
    'statuses',
    'savings-debts',
    'privacy'
  ]);
});

it('все четыре статуса объяснены', () => {
  expect(STATUS_EXPLAINED.map((s) => s.status).sort()).toEqual([
    'debt',
    'paid',
    'planned',
    'unplanned'
  ]);
});

it('горячие клавиши: у каждой строки есть клавиши и описание', () => {
  for (const group of KEY_GROUPS) {
    for (const item of group.items) {
      expect(item.keys.length).toBeGreaterThan(0);
      expect(item.text.trim()).not.toBe('');
    }
  }
});

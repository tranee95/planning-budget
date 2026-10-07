import { expect, it } from 'vitest';
import { HELP_EXTRA, INTRO_STEPS, KEY_GROUPS, STATUS_EXPLAINED } from './content';

it('идентификаторы разделов уникальны, у каждого есть заголовок и текст', () => {
  const all = [...INTRO_STEPS, ...HELP_EXTRA];
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

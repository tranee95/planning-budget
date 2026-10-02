import { describe, expect, it } from 'vitest';
import type { Prefs } from '$lib/api/bindings';
import { applyPrefs } from './prefs.svelte';

const prefs = (over: Partial<Prefs>): Prefs => ({
  theme: 'system',
  locale: 'ru-RU',
  uiScale: 100,
  reducedMotion: 'system',
  autolockMinutes: 5,
  showTips: true,
  ...over
});

describe('applyPrefs', () => {
  it('передаёт масштаб интерфейса и в zoom, и в --ui-zoom: высота на 100vh делится на него', () => {
    const root = document.createElement('html');
    applyPrefs(prefs({ uiScale: 125 }), root);
    expect(root.style.zoom).toBe('1.25');
    expect(root.style.getPropertyValue('--ui-zoom')).toBe('1.25');
    applyPrefs(prefs({ uiScale: 90 }), root);
    expect(root.style.getPropertyValue('--ui-zoom')).toBe('0.9');
  });

  it('при масштабе 100 % возвращает значения по умолчанию', () => {
    const root = document.createElement('html');
    applyPrefs(prefs({ uiScale: 125 }), root);
    applyPrefs(prefs({ uiScale: 100 }), root);
    expect(root.style.zoom).toBe('');
    expect(root.style.getPropertyValue('--ui-zoom')).toBe('');
  });

  it('тема и «меньше анимаций» попадают на корневой элемент', () => {
    const root = document.createElement('html');
    applyPrefs(prefs({ theme: 'dark', reducedMotion: 'on' }), root);
    expect(root.dataset.theme).toBe('dark');
    expect(root.dataset.motion).toBe('reduce');
    applyPrefs(prefs({ theme: 'system', reducedMotion: 'system' }), root);
    expect(root.dataset.theme).toBeUndefined();
    expect(root.dataset.motion).toBeUndefined();
  });
});

describe('applyPrefs: системная тема', () => {
  it('разрешает «системную» тему в явную по теме ОС, которую сообщило окно', () => {
    const root = document.createElement('html');
    applyPrefs(prefs({ theme: 'system' }), root, true);
    expect(root.dataset.theme).toBe('dark');
    applyPrefs(prefs({ theme: 'system' }), root, false);
    expect(root.dataset.theme).toBe('light');
  });

  it('пока тема ОС неизвестна, атрибут не ставится: работает media-запрос из токенов', () => {
    const root = document.createElement('html');
    root.dataset.theme = 'dark';
    applyPrefs(prefs({ theme: 'system' }), root, null);
    expect(root.dataset.theme).toBeUndefined();
  });

  it('явная тема важнее темы ОС', () => {
    const root = document.createElement('html');
    applyPrefs(prefs({ theme: 'light' }), root, true);
    expect(root.dataset.theme).toBe('light');
  });
});

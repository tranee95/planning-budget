import { expect, it } from 'vitest';
import { affectsCategories } from './categories.svelte';

it('affectsCategories: только категории и полная замена данных', () => {
  expect(affectsCategories('categories')).toBe(true);
  expect(affectsCategories('all')).toBe(true);
  expect(affectsCategories('transactions')).toBe(false);
  expect(affectsCategories('limits')).toBe(false);
});

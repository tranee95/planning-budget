import { describe, expect, it } from 'vitest';
import { deviationTone } from './deviation';

describe('deviationTone', () => {
  it('перерасход трат — плохо', () => {
    expect(deviationTone('mandatory', 480_000)).toBe('bad');
    expect(deviationTone('wants', 1)).toBe('bad');
  });
  it('отложить или погасить больше плана — хорошо', () => {
    expect(deviationTone('savings', 530_000)).toBe('good');
    expect(deviationTone('loans', 1)).toBe('good');
  });
  it('равенство и недобор не окрашиваются', () => {
    expect(deviationTone('wants', 0)).toBe('none');
    expect(deviationTone('savings', -100)).toBe('none');
  });
});

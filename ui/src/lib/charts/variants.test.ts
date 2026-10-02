import { describe, expect, it } from 'vitest';
import type { ChartSpecDto } from '$lib/api/bindings';
import { newSpec } from '../../routes/(app)/analytics/builder.svelte';
import { unavailable, variants, withSeries } from './variants';

describe('variants', () => {
  it('covers every switchable field once per value', () => {
    const ids = variants(newSpec()).map((v) => v.id);
    expect(ids).toContain('type:donut');
    expect(ids).toContain('metric:savings_rate');
    expect(ids).toContain('group:tag');
    expect(ids).toContain('series:metric');
    expect(ids).toContain('option:cumulative');
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('does not offer single metrics while series are metrics', () => {
    const spec: ChartSpecDto = withSeries(newSpec(), 'metric');
    expect(variants(spec).some((v) => v.id.startsWith('metric:'))).toBe(false);
  });

  it('series by metric needs two metrics and keeps the first as the main one', () => {
    const spec = withSeries(newSpec(), 'metric');
    expect(spec.seriesBy).toBe('metric');
    expect(spec.metrics).toEqual(['income', 'expenses']);
    expect(spec.metric).toBe('income');
    expect(withSeries(spec, 'none')).toMatchObject({ seriesBy: null, metrics: [] });
  });

  it('toggles an option in its variant without touching the original', () => {
    const spec = newSpec();
    const variant = variants(spec).find((v) => v.id === 'option:percent');
    expect(variant?.spec.options.percent).toBe(true);
    expect(spec.options.percent).toBe(false);
  });
});

describe('unavailable', () => {
  const reasons = { 'type:donut': 'errors.chart.donut', 'type:bar': null };

  it('returns the reason when the draft itself is valid', () => {
    expect(unavailable(reasons, null, 'type:donut')).toBe('errors.chart.donut');
    expect(unavailable(reasons, null, 'type:bar')).toBeNull();
    expect(unavailable(reasons, null, 'type:line')).toBeNull();
  });

  it('does not blame a value for a problem the draft already has', () => {
    expect(unavailable(reasons, 'errors.chart.donut', 'type:donut')).toBeNull();
    expect(unavailable(reasons, 'errors.chart.kpi', 'type:donut')).toBe('errors.chart.donut');
  });
});

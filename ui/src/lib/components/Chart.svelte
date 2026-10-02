<script lang="ts">
  import { onMount } from 'svelte';
  import type { ChartDataDto, ChartSpecDto } from '$lib/api/bindings';
  import { cssReader, resolveColor, type CategoryColors } from '$lib/charts/colors';
  import { drillQuery } from '$lib/charts/drill';
  import { formatValue } from '$lib/charts/format';
  import { toEcharts } from '$lib/charts/toEcharts';
  import ChartTable from './ChartTable.svelte';

  type Props = {
    data: ChartDataDto;
    spec: ChartSpecDto;
    /** Цвета категорий по id: токен `category:<id>` берёт цвет из данных. */
    categoryColors?: CategoryColors;
    /** Клик по столбцу, сектору или точке; параметр — запрос для «Расходов». */
    ondrill?: (query: string) => void;
  };

  type Instance = import('echarts/core').ECharts;

  let { data, spec, categoryColors = new Map(), ondrill }: Props = $props();

  let host = $state<HTMLDivElement>();
  let chart: Instance | undefined;
  let visible = $state(false);

  const plotted = $derived(spec.type !== 'table' && spec.type !== 'kpi');
  const legend = $derived(
    spec.type === 'donut'
      ? data.categories.map((name, i) => ({ name, token: data.categoryTokens[i] ?? '' }))
      : data.series.length > 1
        ? data.series.map((s) => ({ name: s.name, token: s.color }))
        : []
  );
  const headline = $derived.by(() => {
    const value = data.totals?.[0] ?? data.series[0]?.values[0];
    return value === null || value === undefined ? '—' : formatValue(value, data.unit);
  });

  function colorOf(token: string): string {
    return resolveColor(token, cssReader(), categoryColors);
  }

  function reducedMotion(): boolean {
    return (
      document.documentElement.dataset.motion === 'reduce' ||
      matchMedia('(prefers-reduced-motion: reduce)').matches
    );
  }

  /** Цвета canvas берутся из CSS при каждой сборке: смена темы требует новой сборки. */
  function render(): void {
    chart?.setOption(
      toEcharts(data, spec, { css: cssReader(), categoryColors, reducedMotion: reducedMotion() }),
      { notMerge: true }
    );
  }

  $effect(() => {
    render();
  });

  $effect(() => {
    if (!visible || !plotted || host === undefined || chart !== undefined) return;
    const node = host;
    let cancelled = false;
    void import('$lib/charts/echarts').then(({ echarts }) => {
      if (cancelled) return;
      const instance = echarts.init(node, undefined, { renderer: 'canvas' });
      instance.on('click', (params) => {
        if (params.componentType !== 'series' || ondrill === undefined) return;
        const query = drillQuery(spec, data, params.dataIndex, params.seriesIndex ?? 0);
        if (query !== null) ondrill(query);
      });
      chart = instance;
      render();
    });
    return () => {
      cancelled = true;
    };
  });

  onMount(() => {
    const root = document.documentElement;
    const mutations = new MutationObserver(render);
    mutations.observe(root, { attributes: true, attributeFilter: ['data-theme', 'data-motion'] });
    const dark = matchMedia('(prefers-color-scheme: dark)');
    dark.addEventListener('change', render);

    const resize = new ResizeObserver(() => chart?.resize());
    const intersect = new IntersectionObserver(([entry]) => {
      if (entry?.isIntersecting === true) visible = true;
    });
    if (host !== undefined) {
      resize.observe(host);
      intersect.observe(host);
    }
    return () => {
      mutations.disconnect();
      dark.removeEventListener('change', render);
      resize.disconnect();
      intersect.disconnect();
      chart?.dispose();
      chart = undefined;
    };
  });
</script>

<figure>
  {#if spec.type === 'kpi'}
    <p class="kpi">{headline}</p>
  {:else if spec.type === 'table'}
    <ChartTable {data} caption={spec.title} />
  {:else}
    <ChartTable {data} caption={spec.title} hidden />
  {/if}
  <!-- Узел остаётся в разметке при любом типе: наблюдатели и экземпляр ECharts привязаны к нему один раз. -->
  <div bind:this={host} class="plot" hidden={!plotted} role="img" aria-label={spec.title}></div>
  {#if legend.length > 0}
    <ul class="legend">
      {#each legend as item, index (index)}
        <li><i style:background={colorOf(item.token)}></i>{item.name}</li>
      {/each}
    </ul>
  {/if}
</figure>

<style>
  figure {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    height: 100%;
    margin: 0;
    min-height: 0;
  }
  .plot {
    flex: 1;
    min-height: 120px;
  }
  .plot[hidden] {
    display: none;
  }
  .kpi {
    margin: auto 0;
    font-size: var(--fs-44);
    font-weight: var(--fw-bold);
    letter-spacing: var(--tracking-tight);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1) var(--sp-4);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-12);
    color: var(--muted);
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .legend i {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
</style>

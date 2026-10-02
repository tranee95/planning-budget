<script lang="ts">
  import type { MonthSummaryDto } from '$lib/api/bindings';
  import { Card, Segmented } from '$lib/components';
  import { formatMoney, formatMonth } from '$lib/format';
  import type { ChartRange } from '../overview.svelte';

  type Props = {
    months: MonthSummaryDto[];
    range: ChartRange;
    year: number;
    onrange: (range: ChartRange) => void;
  };

  let { months, range, year, onrange }: Props = $props();

  const W = 620;
  const H = 300;
  const LEFT = 44;
  const BOTTOM = 26;
  const plotH = H - BOTTOM;
  const TICKS = 4;

  const ranges = $derived([
    { value: 'year', label: String(year) },
    { value: '12m', label: '12 мес' },
    { value: 'all', label: 'Всё' }
  ] as const);

  const series = [
    { key: 'income', name: 'Доход', fill: 'var(--faint)' },
    { key: 'expenses', name: 'Расходы', fill: 'var(--ink)' },
    { key: 'savings', name: 'Сбережения', fill: 'var(--accent)' }
  ] as const;

  /** Нижняя граница верха шкалы, копейки: на пустых данных подписи оси не должны вырождаться в «0». */
  const MIN_PEAK = 100_000;

  const hasData = $derived(months.some((m) => m.income > 0 || m.expenses > 0 || m.savings > 0));

  /** Верх шкалы — ближайший «круглый» шаг над максимумом ряда. */
  const tickValue = $derived.by(() => {
    const peak = Math.max(MIN_PEAK, ...months.flatMap((m) => [m.income, m.expenses, m.savings]));
    const magnitude = 10 ** Math.floor(Math.log10(peak / TICKS));
    const nice = [1, 2, 2.5, 5, 10].find((k) => k * magnitude * TICKS >= peak) ?? 10;
    return nice * magnitude;
  });
  const top = $derived(tickValue * TICKS);

  const slot = $derived(months.length === 0 ? 0 : (W - LEFT) / months.length);
  const barW = $derived(Math.min(14, slot / 4));
  const y = (kopecks: number): number => plotH - (Math.max(kopecks, 0) / top) * plotH;

  const axisLabel = (kopecks: number): string => {
    const rubles = Math.round(kopecks / 100);
    return rubles >= 1000 ? `${String(Math.round(rubles / 1000))}к` : String(rubles);
  };
</script>

<Card title="Доходы, расходы и сбережения">
  {#snippet actions()}
    <Segmented
      label="Период графика"
      options={ranges}
      value={range}
      onchange={(value: ChartRange) => {
        onrange(value);
      }}
    />
  {/snippet}
  <div class="legend">
    {#each series as s (s.key)}
      <span><i style:background={s.fill}></i>{s.name}</span>
    {/each}
  </div>
  {#if months.length === 0 || !hasData}
    <p class="empty">Данных за выбранный период нет.</p>
  {:else}
    <svg viewBox="0 0 {W} {H}" role="img" aria-label="Доходы, расходы и сбережения по месяцам">
      {#each [0, 1, 2, 3, 4] as i (i)}
        <line x1={LEFT} x2={W} y1={y(i * tickValue)} y2={y(i * tickValue)} />
        <text x={LEFT - 8} y={y(i * tickValue) + 4} text-anchor="end">
          {axisLabel(i * tickValue)}
        </text>
      {/each}
      {#each months as m, i (m.month)}
        {@const cx = LEFT + slot * (i + 0.5)}
        <g>
          <title>
            {formatMonth(m.month)}: доход {formatMoney(m.income)}, расходы {formatMoney(
              m.expenses
            )}, сбережения {formatMoney(m.savings)}
          </title>
          {#each series as s, k (s.key)}
            <rect
              class="bar"
              style:animation-delay="{i * 30}ms"
              x={cx + (k - 1.5) * (barW + 2)}
              y={y(m[s.key])}
              width={barW}
              height={plotH - y(m[s.key])}
              rx="3"
              fill={s.fill}
            />
          {/each}
          <text class="month" x={cx} y={H - 6} text-anchor="middle">
            {formatMonth(m.month, 'short').slice(0, 3)}
          </text>
        </g>
      {/each}
    </svg>
  {/if}
</Card>

<style>
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-4);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .legend i {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  line {
    stroke: var(--line-2);
  }
  text {
    fill: var(--muted);
    font-size: 11px;
  }
  .month {
    fill: var(--muted);
  }
  .bar {
    transform-box: fill-box;
    transform-origin: bottom;
    animation: growY var(--dur-chart) var(--ease-out) both;
  }
  .empty {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  @keyframes growY {
    from {
      transform: scaleY(0);
    }
  }
</style>

<script lang="ts">
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { Card, Chart, IconButton, SimpleTable } from '$lib/components';
  import { formatMoney, formatMonth } from '$lib/format';
  import { getSavingsVm } from '../savings.svelte';

  const vm = getSavingsVm();
  const item = $derived(vm.selected);
</script>

<Card title="Факт по месяцам">
  {#snippet actions()}
    <div class="years">
      <IconButton
        size="sm"
        aria-label="Предыдущий год"
        disabled={vm.year <= vm.minYear}
        onclick={() => {
          void vm.setYear(vm.year - 1);
        }}
      >
        <ChevronLeft size={16} aria-hidden="true" />
      </IconButton>
      <span class="year" aria-live="polite">{vm.data?.year ?? vm.year}</span>
      <IconButton
        size="sm"
        aria-label="Следующий год"
        disabled={vm.year >= vm.maxYear}
        onclick={() => {
          void vm.setYear(vm.year + 1);
        }}
      >
        <ChevronRight size={16} aria-hidden="true" />
      </IconButton>
    </div>
  {/snippet}
  {#if item !== null && vm.data !== null}
    {#if !vm.hasContributions}
      <p class="note">
        В {vm.data.year} году взносов нет: растёт только стартовый баланс за счёт купонов.
      </p>
    {/if}
    <div class="plot">
      <Chart data={item.factChart} type="line" title="Баланс накопления по месяцам" />
    </div>
    <SimpleTable
      caption={`Накопление по месяцам, ${String(vm.data.year)}`}
      headers={['Месяц', 'Взнос', 'Купон', 'Баланс', 'Внесено всего', 'Купоны всего']}
      rows={item.months.map((m) => ({
        key: m.month,
        label: formatMonth(m.month),
        cells: [
          formatMoney(m.savings),
          formatMoney(m.coupon),
          formatMoney(m.balance),
          formatMoney(m.deposited),
          formatMoney(m.couponIncome)
        ]
      }))}
    />
  {/if}
</Card>

<style>
  .years {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
  }
  .year {
    min-width: 48px;
    text-align: center;
    font-variant-numeric: tabular-nums;
    font-weight: var(--fw-medium);
  }
  .plot {
    height: 260px;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>

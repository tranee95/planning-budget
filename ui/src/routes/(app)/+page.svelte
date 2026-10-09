<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Button, CorridorBar, EmptyState, Kpi, Skeleton } from '$lib/components';
  import { month } from '$lib/stores/month.svelte';
  import LimitsList from './_overview/LimitsList.svelte';
  import PlanCard from './_overview/PlanCard.svelte';
  import MonthsChart from './_overview/MonthsChart.svelte';
  import StatusStrip from './_overview/StatusStrip.svelte';
  import { OverviewVm, setOverviewVm, type ChartRange } from './overview.svelte';

  const vm = new OverviewVm();
  setOverviewVm(vm);

  $effect(() => {
    const current = month.current;
    // load читает стор категорий: без untrack эффект перезапускался бы после его загрузки.
    untrack(() => {
      void vm.load(current);
    });
  });

  onMount(() => vm.connect());

  const summary = $derived(vm.summary);
  const delta = $derived(vm.overview?.expensesDeltaPercent ?? null);
</script>

<svelte:head>
  <title>Planning Budget — обзор</title>
</svelte:head>

<div class="screen">
  <p class="subtitle">{vm.subtitle}</p>

  {#if vm.error}
    <EmptyState title="Не удалось загрузить обзор" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load(month.current);
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if summary === null}
    <div class="kpis">
      {#each [0, 1, 2, 3, 4] as i (i)}<Skeleton height="112px" />{/each}
    </div>
  {:else}
    {#if vm.plan !== null}
      <PlanCard
        plan={vm.plan}
        lines={vm.planLines}
        busy={vm.planBusy}
        canCopy={vm.canCopyPlan}
        onlock={() => void vm.lockPlan()}
        onunlock={() => void vm.unlockPlan()}
        oncopy={() => void vm.copyPlan()}
        onrepay={(id: number, paid: boolean) => void vm.payRepayment(id, paid)}
      />
    {/if}

    <div class="kpis">
      <div class="card"><Kpi label="Доход" amount={summary.income} hint={vm.incomeHint} /></div>
      <div class="card">
        <Kpi label="Расходы" amount={summary.expenses}>
          {#if delta !== null}
            <span class="hint">
              <span class="delta" class:good={delta <= 0}
                >{delta > 0 ? '+' : delta < 0 ? '−' : ''}{Math.abs(delta)}%</span
              >
              к среднему за год
            </span>
          {/if}
        </Kpi>
      </div>
      <div class="card">
        <Kpi label="Сбережения" amount={summary.savings}>
          {#if vm.settings !== null && summary.savingsRateBp !== null}
            <CorridorBar
              value={summary.savingsRateBp}
              min={vm.settings.savingsMinBp}
              max={vm.settings.savingsMaxBp}
            />
          {/if}
          <span class="hint">{vm.savingsHint}</span>
        </Kpi>
      </div>
      <div class="card">
        <Kpi label="Свободный остаток" amount={summary.free} hint="после трат и сбережений" />
      </div>
      <div class="card">
        <Kpi label="На неделю" amount={summary.perWeek} hint="остаток ÷ недели месяца" />
      </div>
    </div>

    <StatusStrip byStatus={summary.byStatus} />

    <div class="main">
      <MonthsChart
        months={vm.series}
        range={vm.range}
        year={vm.year?.year ?? Number(month.current.slice(0, 4))}
        onrange={(range: ChartRange) => {
          void vm.setRange(range);
        }}
      />
      <LimitsList lines={vm.limitLines} />
    </div>
  {/if}
</div>

<style>
  .screen {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .subtitle {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: var(--sp-4);
  }
  /* Пять карточек без «сироты» в последнем ряду: 3 + 2, на узкой ширине 2 + 2 + 1. */
  @container (max-width: 880px) {
    .kpis {
      grid-template-columns: repeat(6, minmax(0, 1fr));
    }
    .kpis > :nth-child(-n + 3) {
      grid-column: span 2;
    }
    .kpis > :nth-child(n + 4) {
      grid-column: span 3;
    }
  }
  @container (max-width: 520px) {
    .kpis {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .kpis > * {
      grid-column: span 1;
    }
    .kpis > :last-child {
      grid-column: 1 / -1;
    }
  }
  .card {
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .card :global(.kpi) {
    gap: var(--sp-2);
  }
  .hint {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .delta {
    font-weight: var(--fw-semibold);
    color: var(--unpl);
  }
  .delta.good {
    color: var(--paid);
  }
  .main {
    display: grid;
    grid-template-columns: minmax(0, 1.45fr) minmax(0, 1fr);
    gap: var(--sp-4);
    align-items: start;
  }
  @media (max-width: 1100px) {
    .main {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

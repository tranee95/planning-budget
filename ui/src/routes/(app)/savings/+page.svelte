<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, EmptyState, Kpi, Skeleton } from '$lib/components';
  import { formatMoney } from '$lib/format';
  import AccumulationList from './_components/AccumulationList.svelte';
  import AccumulationParams from './_components/AccumulationParams.svelte';
  import BrokerSoon from './_components/BrokerSoon.svelte';
  import SavingsFact from './_components/SavingsFact.svelte';
  import SavingsForecast from './_components/SavingsForecast.svelte';
  import { SavingsVm, setSavingsVm } from './savings.svelte';

  const vm = new SavingsVm();
  setSavingsVm(vm);

  onMount(() => {
    void vm.load();
    return vm.connect();
  });

  const data = $derived(vm.data);
  const optimistic = $derived(data?.totalScenarios.find((s) => s.scenario === 'a') ?? null);
</script>

<svelte:head>
  <title>Planning Budget — сбережения</title>
</svelte:head>

<div class="screen">
  {#if vm.error}
    <EmptyState title="Не удалось построить расчёт" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load();
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if data === null}
    <Skeleton height="420px" />
  {:else}
    <div class="kpis">
      <div class="card">
        <Kpi
          label="Баланс накоплений"
          amount={data.totalBalance}
          hint={`на конец ${String(data.year)} года · ${String(data.items.length)} накопл.`}
        />
      </div>
      <div class="card">
        <Kpi label="План взноса в месяц" amount={data.monthPlan} hint="сумма планов накоплений" />
      </div>
      <div class="card">
        <Kpi
          label="Прогноз через 5 лет ≈"
          value={optimistic === null ? '—' : formatMoney(optimistic.y5)}
          hint="сценарий A · не гарантирован"
        />
      </div>
    </div>

    <div class="body">
      <AccumulationList />
      <div class="detail">
        <AccumulationParams />
        <SavingsFact />
        <SavingsForecast />
      </div>
    </div>
  {/if}
  <BrokerSoon />
</div>

<style>
  .screen {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding-bottom: var(--sp-6);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sp-4);
  }
  .card {
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .body {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    gap: var(--sp-4);
    align-items: start;
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    min-width: 0;
  }
  @container (max-width: 900px) {
    .kpis,
    .body {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

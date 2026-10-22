<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Button, Chart, EmptyState, Kpi, Modal, Skeleton } from '$lib/components';
  import { formatMoney, formatMonth, formatMonthLower } from '$lib/format';
  import { FORMULAS } from '$lib/formulas';
  import { undoLabel } from '$lib/shortcuts';
  import { month } from '$lib/stores/month.svelte';
  import SavingsList from './_components/SavingsList.svelte';
  import { SavingsVm, setSavingsVm } from './savings.svelte';

  const vm = new SavingsVm();
  setSavingsVm(vm);

  $effect(() => {
    const current = month.current;
    untrack(() => {
      void vm.load(current);
    });
  });

  /** Линия плана месяца поверх столбцов: сколько нужно отложить (копейки → рубли, как в данных графика). */
  const chartData = $derived(
    vm.progress === null
      ? null
      : {
          ...vm.progress.chart,
          referenceLines:
            vm.planTotal > 0 ? [{ name: 'План месяца', from: vm.planTotal / 100, to: null }] : []
        }
  );

  onMount(() => vm.connect());
</script>

<svelte:head>
  <title>Planning Budget — сбережения</title>
</svelte:head>

<div class="screen">
  {#if vm.error}
    <EmptyState title="Не удалось загрузить сбережения" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load(month.current);
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.loading && vm.progress === null}
    <Skeleton height="240px" />
  {:else}
    <div class="kpis">
      <div class="card">
        <Kpi
          label="Откладываю в месяц"
          amount={vm.planTotal}
          hint={`план на ${formatMonthLower(vm.month)}`}
          formula={FORMULAS.savingsPlan}
        />
      </div>
      <div class="card">
        <Kpi
          label="Отложено в месяце"
          amount={vm.monthTotal}
          hint={formatMonth(vm.month)}
          formula={FORMULAS.savingsMonth}
        />
      </div>
      {#if vm.progress}
        <div class="card">
          <Kpi
            label="Накоплено всего"
            amount={vm.progress.total}
            hint="за весь период"
            formula={FORMULAS.savingsTotal}
          />
        </div>
      {/if}
    </div>
    {#if vm.progress && chartData && vm.progress.months.length > 0}
      <div class="card chart">
        <h2 class="chart-title">Отложено по месяцам</h2>
        <div class="chart-body">
          <Chart
            data={chartData}
            type="stacked_bar"
            title="Отложено по месяцам"
            categoryColors={vm.chartColors}
          />
        </div>
      </div>
    {/if}
    <SavingsList />
  {/if}
</div>

{#if vm.closeAsk}
  <Modal
    title="Закрыть цель?"
    width={460}
    onclose={() => {
      vm.cancelClose();
    }}
  >
    <p class="ask">
      Цель «{vm.closeAsk.name}» уйдёт из списка накоплений. Отложенные суммы останутся в «Накоплено
      всего» и на графике, вернуть цель можно через «Показать закрытые цели».
    </p>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          vm.cancelClose();
        }}>Отмена</Button
      >
      <Button
        variant="primary"
        onclick={() => {
          void vm.confirmClose();
        }}>Закрыть цель</Button
      >
    {/snippet}
  </Modal>
{/if}

{#if vm.deleteAsk}
  <Modal
    title="Удалить цель?"
    width={460}
    onclose={() => {
      vm.cancelDelete();
    }}
  >
    <p class="ask">
      Цель «{vm.deleteAsk.name}» и все её суммы ({formatMoney(vm.deleteAsk.total)}) пропадут из
      списка, итогов и графика. Сразу после удаления можно нажать «Вернуть» или {undoLabel}.
    </p>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          vm.cancelDelete();
        }}>Отмена</Button
      >
      <Button
        variant="danger"
        onclick={() => {
          void vm.confirmDelete();
        }}>Удалить цель</Button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .ask {
    margin: 0;
    font-size: var(--fs-13);
    line-height: var(--lh-body);
  }
  .screen {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding-bottom: var(--sp-6);
  }
  .card {
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--sp-4);
  }
  .chart {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    height: 264px;
  }
  .chart-title {
    margin: 0;
    font-size: var(--fs-14);
    font-weight: var(--fw-semibold);
  }
  .chart-body {
    flex: 1;
    min-height: 0;
  }
</style>

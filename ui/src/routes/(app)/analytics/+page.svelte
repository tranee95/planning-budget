<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { Button, EmptyState, Skeleton } from '$lib/components';
  import { categories } from '$lib/stores/categories.svelte';
  import { AnalyticsVm, setAnalyticsVm } from './analytics.svelte';
  import BuilderPanel from './_components/BuilderPanel.svelte';
  import { BuilderVm, setBuilderVm } from './builder.svelte';
  import DashboardBar from './_components/DashboardBar.svelte';
  import DashboardGrid from './_components/DashboardGrid.svelte';

  const vm = new AnalyticsVm();
  setAnalyticsVm(vm);
  const builder = new BuilderVm();
  setBuilderVm(builder);

  function drill(query: string): void {
    // Путь разрешён через resolve, запрос дописывается к нему.
    // eslint-disable-next-line svelte/no-navigation-without-resolve
    void goto(`${resolve('/expenses')}?q=${encodeURIComponent(query)}`);
  }

  onMount(() => {
    void categories.ensure();
    void vm.load();
    const stopCategories = categories.watch();
    const stopData = vm.connect();
    return () => {
      stopCategories();
      stopData();
    };
  });
</script>

<svelte:head>
  <title>Planning Budget — аналитика</title>
</svelte:head>

<div class="screen">
  <DashboardBar />
  {#if vm.error}
    <EmptyState title="Не удалось загрузить дашборды" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load();
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.loading && vm.cards.length === 0}
    <Skeleton height="360px" />
  {:else if vm.dashboards.length === 0}
    <EmptyState
      title="Дашбордов нет"
      hint="Стандартный дашборд «Мой бюджет» содержит 9 графиков: доходы и расходы, сбережения, траты по статусам и категориям, лимиты."
    >
      {#snippet actions()}
        <Button
          onclick={() => {
            void vm.createDefaultDashboard();
          }}>Создать стандартный</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.cards.length === 0}
    <EmptyState
      title="На дашборде пока нет графиков"
      hint="Графики появятся здесь, когда вы их добавите."
    />
  {:else}
    <DashboardGrid ondrill={drill} />
  {/if}
</div>

{#if builder.open}
  <BuilderPanel />
{/if}

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding-bottom: var(--sp-6);
  }
</style>

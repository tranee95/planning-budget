<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, EmptyState, Skeleton } from '$lib/components';
  import { BondsVm, setBondsVm } from './bonds.svelte';
  import BondsFact from './_components/BondsFact.svelte';
  import BondsForecast from './_components/BondsForecast.svelte';
  import BondsParams from './_components/BondsParams.svelte';
  import BrokerSoon from './_components/BrokerSoon.svelte';

  const vm = new BondsVm();
  setBondsVm(vm);

  onMount(() => {
    void vm.load();
    return vm.connect();
  });
</script>

<svelte:head>
  <title>Private Budget — облигации</title>
</svelte:head>

<div class="screen">
  {#if vm.settings !== null}
    <BondsParams />
  {/if}
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
  {:else if vm.data === null}
    <Skeleton height="420px" />
  {:else}
    <BondsFact />
    <BondsForecast />
  {/if}
  <BrokerSoon />
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding-bottom: var(--sp-6);
  }
</style>

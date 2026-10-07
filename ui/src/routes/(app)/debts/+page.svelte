<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { Button, EmptyState, Kpi, Skeleton, Switch } from '$lib/components';
  import { formatMoney, formatMonth } from '$lib/format';
  import { month } from '$lib/stores/month.svelte';
  import DebtList from './_components/DebtList.svelte';
  import DebtSheet from './_components/DebtSheet.svelte';
  import { DebtsVm, setDebtsVm } from './debts.svelte';

  const vm = new DebtsVm();
  setDebtsVm(vm);

  $effect(() => {
    const current = month.current;
    untrack(() => {
      void vm.load(current);
    });
  });

  // График пересчитывает Rust, когда меняется то, от чего он зависит.
  $effect(() => {
    if (vm.scheduleKey) {
      untrack(() => {
        void vm.refreshSchedule();
      });
    }
  });

  onMount(() => {
    // Ссылка «Создать долг» из расходов: ?fromTx=<id>&month=YYYY-MM.
    const id = Number(page.url.searchParams.get('fromTx'));
    const txMonth = page.url.searchParams.get('month');
    if (Number.isInteger(id) && id > 0 && txMonth !== null) {
      void vm.startFromTransaction(id, txMonth);
      void goto(resolve('/debts'), { replaceState: true });
    }
    return vm.connect();
  });

  const overview = $derived(vm.overview);
</script>

<svelte:head>
  <title>Planning Budget — долги</title>
</svelte:head>

<div class="screen">
  <div class="bar">
    <p class="subtitle">{vm.subtitle}</p>
    <div class="closed">
      <Switch
        checked={vm.includeClosed}
        onchange={(value: boolean) => {
          void vm.setIncludeClosed(value);
        }}>Показать закрытые</Switch
      >
    </div>
    <Button
      variant="primary"
      onclick={() => {
        vm.startNew();
      }}>+ Долг</Button
    >
  </div>

  {#if vm.error}
    <EmptyState title="Не удалось загрузить долги" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load(month.current);
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if overview === null}
    <div class="kpis">
      {#each [0, 1, 2] as i (i)}<Skeleton height="96px" />{/each}
    </div>
  {:else}
    <div class="kpis">
      <div class="card">
        <Kpi
          label="Остаток долгов"
          amount={overview.remainingTotal}
          hint={`${String(overview.openCount)} открытых`}
        />
      </div>
      <div class="card">
        <Kpi
          label={`К оплате: ${formatMonth(month.current)}`}
          amount={overview.paymentsPlanned}
          hint={`оплачено ${formatMoney(overview.paymentsPaid)}`}
        />
      </div>
      <div class="card">
        <Kpi label="Закрыто" value={String(overview.closedCount)} hint="за всё время" />
      </div>
    </div>

    {#if overview.debts.length === 0}
      <EmptyState
        title={overview.closedCount > 0 ? 'Открытых долгов нет' : 'Долгов пока нет'}
        hint="Долг можно создать вручную или из траты со статусом «Долг»."
      >
        {#snippet actions()}
          <Button
            variant="primary"
            onclick={() => {
              vm.startNew();
            }}>Добавить долг</Button
          >
        {/snippet}
      </EmptyState>
    {:else}
      <DebtList
        debts={overview.debts}
        selected={vm.selected}
        onselect={(id: number) => {
          vm.select(id);
        }}
      />
    {/if}
  {/if}
</div>

{#if vm.selected !== null}
  <DebtSheet />
{/if}

<style>
  .screen {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
  }
  .subtitle {
    flex: 1;
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .closed {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sp-4);
  }
  .card {
    padding: var(--sp-4) var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  @container (max-width: 720px) {
    .kpis {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { Button, EmptyState, FilterBar, Segmented, Skeleton, StatusDot } from '$lib/components';
  import { STATUS_LABEL, STATUS_ORDER } from '$lib/components/status';
  import { formatMoney } from '$lib/format';
  import { month } from '$lib/stores/month.svelte';
  import CategoryCard from './_components/CategoryCard.svelte';
  import QuickAdd from './_components/QuickAdd.svelte';
  import ExpensesTable from './_components/ExpensesTable.svelte';
  import { ExpensesVm, setExpensesVm } from './expenses.svelte';

  const vm = new ExpensesVm();
  setExpensesVm(vm);

  const MODES = [
    { value: 'blocks', label: 'Блоки' },
    { value: 'table', label: 'Таблица' }
  ] as const;

  // Ссылки палитры: `?q=` открывает таблицу с запросом, `?edit=` — правку траты.
  $effect(() => {
    const params = page.url.searchParams;
    const query = params.get('q');
    const edit = params.get('edit');
    if (query === null && edit === null) return;
    untrack(() => {
      if (query !== null) {
        vm.mode = 'table';
        vm.filter.set(query);
      }
      if (edit !== null) vm.openForEdit(Number(edit));
      void goto(resolve('/expenses'), { replaceState: true, noScroll: true, keepFocus: true });
    });
  });

  $effect(() => {
    // Месяц, запрос и режим — зависимости эффекта; `load` читает их же и стор категорий под untrack.
    const [current] = [month.current, vm.filter.applied, vm.mode];
    // load читает состояние стора категорий: без untrack эффект перезапускался бы после их загрузки.
    untrack(() => {
      void vm.load(current);
    });
  });

  onMount(() => vm.connect());
</script>

<svelte:head>
  <title>Planning Budget — расходы</title>
</svelte:head>

<div class="screen">
  <div class="bar">
    <span class="lbl">Показать</span>
    {#each STATUS_ORDER as status (status)}
      <button
        type="button"
        class="chip {status}"
        aria-pressed={vm.statusFilter.has(status)}
        onclick={() => {
          vm.toggleStatus(status);
        }}
      >
        <span aria-hidden="true" class="dot"><StatusDot {status} size={12} /></span>{STATUS_LABEL[
          status
        ]}
      </button>
    {/each}
    <span class="spacer"></span>
    {#if vm.totals}
      <span class="num totals">
        Расходы <b>{formatMoney(vm.totals.spent)}</b> · лимиты
        <b>{formatMoney(vm.totals.limits)}</b>
        ·
        <b class:over={vm.totals.remaining < 0}>
          {vm.totals.remaining < 0 ? '−' : ''}{formatMoney(Math.abs(vm.totals.remaining))}
        </b>
      </span>
    {/if}
    <Segmented
      label="Режим отображения"
      options={MODES}
      value={vm.mode}
      onchange={(mode: 'blocks' | 'table') => {
        vm.mode = mode;
      }}
    />
  </div>

  {#if vm.mode === 'table'}
    <FilterBar
      filter={vm.filter}
      subject="трат"
      placeholder="Поиск по тратам: лента статус:оплачено сумма>5000 период:июн..сен #тег"
    />
  {/if}

  {#if vm.error}
    <EmptyState title="Не удалось загрузить расходы" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load(month.current);
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.loading && vm.overview === null}
    <div class="grid">
      {#each [0, 1, 2, 3] as i (i)}<Skeleton height="220px" />{/each}
    </div>
  {:else if vm.mode === 'blocks'}
    <div class="grid">
      {#each vm.blocks as block, index (block.category.id)}
        <CategoryCard {block} {index} />
      {/each}
    </div>
  {:else}
    <ExpensesTable />
  {/if}
</div>

{#if vm.editorOpen}
  <QuickAdd />
{/if}

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
  }
  .lbl {
    margin-right: var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .dot {
    display: inline-flex;
  }
  .spacer {
    flex-grow: 1;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    height: 26px;
    padding-inline: var(--sp-3);
    border: 1px solid transparent;
    border-radius: var(--r-full);
    font: inherit;
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
    cursor: pointer;
    border-color: var(--line);
    background: var(--surface);
    color: var(--ink);
  }
  .chip[aria-pressed='true'] {
    border-color: currentColor;
  }
  .chip[aria-pressed='true'].paid {
    background: var(--paid-bg);
    color: var(--paid);
  }
  .chip[aria-pressed='true'].debt {
    background: var(--debt-bg);
    color: var(--debt);
  }
  .chip[aria-pressed='true'].unplanned {
    background: var(--unpl-bg);
    color: var(--unpl);
  }
  .chip[aria-pressed='true'].planned {
    background: var(--plan-bg);
    color: var(--plan);
  }
  .totals {
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .totals b {
    color: var(--ink);
    font-weight: var(--fw-semibold);
  }
  .totals b.over {
    color: var(--unpl);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--sp-4);
    align-content: start;
  }
  @media (max-width: 1280px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  @media (max-width: 960px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>

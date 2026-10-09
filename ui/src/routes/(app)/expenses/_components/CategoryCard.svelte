<script lang="ts">
  import { IconButton, Progress } from '$lib/components';
  import { formatMoney } from '$lib/format';
  import { rise, stagger } from '$lib/motion';
  import { getExpensesVm, type CategoryBlock } from '../expenses.svelte';
  import AddRow from './AddRow.svelte';
  import TxRow from './TxRow.svelte';

  let { block, index }: { block: CategoryBlock; index: number } = $props();

  const vm = getExpensesVm();
  const limit = $derived(block.limit);
  const fact = $derived(limit?.fact ?? 0);
  // Доли лимита для полосы приходят готовыми из Rust (шкала 0…1).
  const paidShare = $derived(limit?.paidUsage ?? 0);
  const plannedShare = $derived(limit?.plannedUsage ?? 0);
</script>

<article
  class="card"
  style:--category-color={block.category.color}
  in:rise={{ delay: stagger(index) }}
>
  <header>
    <div class="head">
      <h2>{block.category.name}</h2>
      {#if block.category.note}<span class="note">{block.category.note}</span>{/if}
    </div>
    <IconButton
      aria-label="Добавить трату в {block.category.name}"
      onclick={() => {
        vm.startAdd(block.category.id);
      }}
    >
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
        <path d="M12 5v14M5 12h14" />
      </svg>
    </IconButton>
  </header>

  <div class="sums">
    <span class="num total">{formatMoney(fact)}</span>
    <div class="sub num">
      {#if limit?.limit != null}
        <span>из {formatMoney(limit.limit)}</span>
        {#if limit.remaining !== null}
          <span class:over={limit.remaining < 0}>
            {limit.remaining < 0 ? 'сверх' : 'ост.'}
            {formatMoney(Math.abs(limit.remaining))}
          </span>
        {/if}
      {:else}
        <span>без лимита</span>
      {/if}
    </div>
  </div>

  {#if limit?.limit != null}
    <Progress value={paidShare} planned={plannedShare} label="Лимит {block.category.name}" />
  {/if}

  <ul>
    {#each block.rows as row (row.id)}
      <TxRow
        {row}
        editing={vm.editingId === row.id}
        onedit={(id: number) => {
          vm.startEdit(id);
        }}
        oncycle={(id: number) => {
          void vm.cycleStatus(id);
        }}
        onsave={(patch: { title: string; amount: number }) => {
          void vm.saveEdit(patch);
        }}
        oncancel={() => {
          vm.cancelEdit();
        }}
      />
    {/each}
    {#if vm.addingTo === block.category.id}
      <AddRow
        categoryName={block.category.name}
        onsubmit={(draft: { title: string; amount: number }) => {
          void vm.commitAdd(draft);
        }}
        oncancel={() => {
          vm.cancelAdd();
        }}
      />
    {/if}
  </ul>
</article>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-width: 0;
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  /* Полоса цвета категории: декоративная, название категории есть в заголовке. */
  .card::before {
    content: '';
    position: absolute;
    inset: 0 var(--r-lg) auto;
    height: 3px;
    border-radius: 0 0 var(--r-xs) var(--r-xs);
    background: var(--category-color);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
  }
  .head {
    display: flex;
    flex-direction: column;
    flex-grow: 1;
    min-width: 0;
  }
  h2 {
    margin: 0;
    overflow: hidden;
    font-size: var(--fs-14);
    font-weight: var(--fw-semibold);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .sums {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .total {
    font-size: var(--fs-22);
    font-weight: var(--fw-semibold);
  }
  .sub {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .over {
    color: var(--unpl);
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>

<script lang="ts">
  import type { DebtDto } from '$lib/api/bindings';
  import { Progress } from '$lib/components';
  import { formatMoney, formatMonth } from '$lib/format';
  import { categories } from '$lib/stores/categories.svelte';

  type Props = {
    debts: readonly DebtDto[];
    selected: number | 'new' | null;
    onselect: (id: number) => void;
  };

  let { debts, selected, onselect }: Props = $props();
</script>

<ul>
  {#each debts as debt (debt.id)}
    <li>
      <button
        type="button"
        class="debt"
        class:on={selected === debt.id}
        class:closed={debt.closed}
        onclick={() => {
          onselect(debt.id);
        }}
      >
        <span class="head">
          <span class="name">{debt.lender}</span>
          {#if debt.categoryId !== null}
            <span class="chip">{categories.byId.get(debt.categoryId)?.name ?? ''}</span>
          {/if}
          <span class="rest num">{formatMoney(debt.remaining)}</span>
        </span>
        <Progress value={debt.paidBp / 10_000} label="{debt.lender}: погашено" />
        <span class="foot">
          <span>взято {formatMoney(debt.amount)} · {formatMonth(debt.takenMonth)}</span>
          <span>
            {#if debt.nextPayment}
              следующий платёж: {formatMonth(debt.nextPayment.month)} · {formatMoney(
                debt.nextPayment.amount
              )}
            {:else}
              погашен
            {/if}
          </span>
        </span>
      </button>
    </li>
  {/each}
</ul>

<style>
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .debt {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    width: 100%;
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .debt:hover {
    border-color: var(--faint);
  }
  .debt.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .debt.closed {
    opacity: 0.7;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }
  .name {
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .chip {
    padding: 1px var(--sp-2);
    border-radius: var(--r-full);
    background: var(--line-2);
    color: var(--ink-2);
    font-size: var(--fs-12);
  }
  .rest {
    margin-inline-start: auto;
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  .foot {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-12);
  }
</style>

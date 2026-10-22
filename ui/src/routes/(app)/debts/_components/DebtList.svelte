<script lang="ts">
  import type { DebtDto } from '$lib/api/bindings';
  import { Progress } from '$lib/components';
  import { formatMoney, formatMonthLower } from '$lib/format';
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
            {@const category = categories.byId.get(debt.categoryId)}
            <span class="chip">
              <span class="swatch" style:background={category?.color ?? 'var(--faint)'}></span>
              {category?.name ?? ''}
            </span>
          {/if}
          <span class="rest num">{formatMoney(debt.remaining)}</span>
        </span>
        <Progress value={debt.paidBp / 10_000} label="{debt.lender}: погашено" kind="goal" />
        <span class="paid-line">
          <span class="num"
            >оплачено {formatMoney(debt.amount - debt.remaining)} из {formatMoney(
              debt.amount
            )}</span
          >
          {#if debt.payments.length > 1}
            <!-- по точке на каждый платёж графика: закрашена — оплачен -->
            <span
              class="dots"
              role="img"
              aria-label="Платежей оплачено {debt.payments.filter((p) => p.status === 'paid')
                .length} из {debt.payments.length}"
            >
              {#each debt.payments as p (p.id)}<span class="dot" class:on={p.status === 'paid'}
                ></span>{/each}
            </span>
          {/if}
        </span>
        <span class="foot">
          <span>взято {formatMoney(debt.amount)} · {formatMonthLower(debt.takenMonth)}</span>
          <span>
            {#if debt.nextPayment}
              следующий платёж: {formatMonthLower(debt.nextPayment.month)} · {formatMoney(
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
  .swatch {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-inline-end: var(--sp-1);
    border-radius: 2px;
  }
  .paid-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .dots {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border: 1.5px solid var(--faint);
    border-radius: var(--r-full);
  }
  .dot.on {
    border-color: var(--paid-bar);
    background: var(--paid-bar);
  }
</style>

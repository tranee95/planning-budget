<script lang="ts">
  import { resolve } from '$app/paths';
  import { formatMoney, formatPercent } from '$lib/format';
  import { getSavingsVm } from '../savings.svelte';

  const vm = getSavingsVm();
  const items = $derived(vm.data?.items ?? []);
</script>

<section class="list" aria-labelledby="accumulations-title">
  <h2 id="accumulations-title">Накопления</h2>
  <ul>
    {#each items as item (item.categoryId)}
      <li>
        <button
          type="button"
          class="item"
          class:on={vm.selectedId === item.categoryId}
          aria-pressed={vm.selectedId === item.categoryId}
          onclick={() => {
            vm.select(item.categoryId);
          }}
        >
          <span class="head">
            <span class="name">{vm.nameOf(item.categoryId)}</span>
            <span class="balance num">{formatMoney(item.balance)}</span>
          </span>
          <span class="meta">
            <span>
              {#if item.planKind === 'fixed' && item.planFixed !== null}
                {formatMoney(item.planFixed)} в месяц
              {:else}
                {formatPercent(item.planRateBp)} дохода · {formatMoney(item.plan)}
              {/if}
            </span>
            <span>
              {item.params.annualRateBp === 0
                ? 'без процентов'
                : `${formatPercent(item.params.annualRateBp)} · налог ${formatPercent(item.params.taxBp)}`}
            </span>
          </span>
        </button>
      </li>
    {:else}
      <li class="empty">Накоплений пока нет.</li>
    {/each}
  </ul>
  <p class="hint">
    Накопление — категория типа «Сбережения». Новые добавляются в <a href={resolve('/categories')}
      >Категориях</a
    >.
  </p>
</section>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    width: 100%;
    padding: var(--sp-3) var(--sp-4);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    border-color: var(--faint);
  }
  .item.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-2);
  }
  .name {
    font-size: var(--fs-14);
    font-weight: var(--fw-semibold);
  }
  .balance {
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  .meta {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-2);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .empty,
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .hint a {
    display: inline-block;
    min-height: 24px;
    line-height: 24px;
    color: var(--accent);
    text-decoration: underline;
  }
</style>

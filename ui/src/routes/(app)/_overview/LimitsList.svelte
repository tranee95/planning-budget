<script lang="ts">
  import { resolve } from '$app/paths';
  import { Card, Progress } from '$lib/components';
  import { formatMoney } from '$lib/format';
  import type { LimitLine } from '../overview.svelte';

  let { lines }: { lines: LimitLine[] } = $props();
</script>

<Card title="Лимиты месяца">
  {#snippet actions()}
    <a href={resolve('/expenses')}>Все категории →</a>
  {/snippet}
  {#if lines.length === 0}
    <p class="empty">Лимиты не заданы. Их можно назначить в разделе «Категории».</p>
  {:else}
    <ul>
      {#each lines as { category, row } (category.id)}
        <li>
          <div class="head">
            <span class="name">{category.name}</span>
            <span class="num amount">
              {formatMoney(row.fact)} <span class="of">/ {formatMoney(row.limit ?? 0)}</span>
            </span>
            <span class="num pct {row.level ?? 'ok'}">{row.usagePercent ?? 0}%</span>
          </div>
          <Progress value={row.usage ?? 0} label="{category.name}: использование лимита" />
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    font-size: var(--fs-13);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-weight: var(--fw-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .amount {
    color: var(--muted);
  }
  .of {
    color: var(--muted);
  }
  .pct {
    width: 44px;
    font-size: var(--fs-12);
    font-weight: var(--fw-semibold);
    color: var(--muted);
    text-align: right;
  }
  .pct.over {
    color: var(--unpl);
  }
  .empty {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  a {
    display: inline-flex;
    align-items: center;
    min-height: 32px;
    color: var(--accent);
    font-size: var(--fs-13);
    font-weight: var(--fw-medium);
  }
</style>

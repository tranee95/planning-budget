<script lang="ts">
  import Calendar from '@lucide/svelte/icons/calendar';
  import Receipt from '@lucide/svelte/icons/receipt';
  import Search from '@lucide/svelte/icons/search';
  import Table from '@lucide/svelte/icons/table';
  import Tag from '@lucide/svelte/icons/tag';
  import Wallet from '@lucide/svelte/icons/wallet';
  import Zap from '@lucide/svelte/icons/zap';
  import type { PaletteEntry } from '$lib/palette/entries';
  import Kbd from './Kbd.svelte';
  import StatusChip from './StatusChip.svelte';

  type Props = {
    id: string;
    entry: PaletteEntry;
    active: boolean;
    onpick: () => void;
    onhover: () => void;
  };

  let { id, entry, active, onpick, onhover }: Props = $props();

  const icons = {
    transaction: Receipt,
    income: Wallet,
    category: Tag,
    month: Calendar,
    table: Table,
    recent: Search,
    command: Zap
  } as const;
  const Icon = $derived(icons[entry.target.type]);
</script>

<li
  {id}
  role="option"
  aria-selected={active}
  class:active
  onpointermove={onhover}
  onclick={onpick}
  onkeydown={null}
>
  <span class="icon"><Icon size={16} aria-hidden="true" /></span>
  <span class="label">{entry.label}</span>
  {#if entry.meta}<span class="meta">{entry.meta}</span>{/if}
  <span class="spacer"></span>
  {#if entry.status}
    <StatusChip status={entry.status} />
  {:else if entry.note}
    <span class="income">{entry.note}</span>
  {/if}
  {#if entry.amount}<span class="amount">{entry.amount}</span>{/if}
  {#if entry.shortcut}<Kbd>{entry.shortcut}</Kbd>{/if}
  {#if active}<Kbd>↵</Kbd>{/if}
</li>

<style>
  li {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    min-height: 44px;
    padding-inline: var(--sp-3);
    border-radius: var(--r-md);
    font-size: var(--fs-14);
    cursor: pointer;
  }
  li.active {
    background: var(--accent-soft);
  }
  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--r-sm);
    background: var(--line-2);
    color: var(--muted);
  }
  .label {
    min-width: 0;
    overflow: hidden;
    font-weight: var(--fw-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    flex: none;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .spacer {
    flex: 1;
  }
  .income {
    flex: none;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .amount {
    flex: none;
    font-weight: var(--fw-medium);
    font-variant-numeric: tabular-nums;
  }
</style>

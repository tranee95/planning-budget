<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import Kbd from '$lib/components/Kbd.svelte';
  import MonthSwitcher from '$lib/components/MonthSwitcher.svelte';
  import { shortcutLabel } from '$lib/shortcuts';

  type Props = { title: string; showMonth: boolean; onsearch: () => void };

  let { title, showMonth, onsearch }: Props = $props();

  const searchKey = shortcutLabel({ code: 'KeyK', mod: true });
</script>

<header class="topbar">
  <h1>{title}</h1>
  {#if showMonth}<MonthSwitcher />{/if}
  <button type="button" class="search" aria-keyshortcuts="Control+K" onclick={onsearch}>
    <Search size={16} aria-hidden="true" />
    <span>Поиск и команды</span>
    <Kbd>{searchKey}</Kbd>
  </button>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--sp-5);
    height: var(--topbar-h);
    padding-inline: var(--sp-8);
  }
  h1 {
    flex: 1;
    margin: 0;
    font-size: var(--fs-22);
    font-weight: var(--fw-semibold);
    letter-spacing: var(--tracking-tight);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 260px;
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--muted);
    font: var(--fs-14) var(--font-sans);
    cursor: pointer;
  }
  .search span {
    flex: 1;
    text-align: left;
  }
  .search:hover {
    background: var(--surface-2);
  }
  .search:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

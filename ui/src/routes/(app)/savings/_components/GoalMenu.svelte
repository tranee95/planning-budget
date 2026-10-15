<script lang="ts">
  type Item = { label: string; danger?: boolean; disabled?: boolean; run: () => void };
  type Props = { name: string; items: Item[] };

  let { name, items }: Props = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();

  function close(refocus: boolean): void {
    open = false;
    if (refocus) trigger?.focus();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) {
      event.stopPropagation();
      close(true);
      return;
    }
    if (!open || (event.key !== 'ArrowDown' && event.key !== 'ArrowUp')) return;
    event.preventDefault();
    const buttons = [
      ...(root?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:enabled') ?? [])
    ];
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === 'ArrowDown' ? at + 1 : at - 1;
    buttons[(next + buttons.length) % buttons.length]?.focus();
  }

  function onWindowClick(event: MouseEvent): void {
    if (open && root && !root.contains(event.target as Node)) close(false);
  }

  function pick(item: Item): void {
    close(true);
    item.run();
  }

  $effect(() => {
    if (open) root?.querySelector<HTMLButtonElement>('[role="menuitem"]:enabled')?.focus();
  });
</script>

<svelte:window onclick={onWindowClick} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="menu" bind:this={root} onkeydown={onKeydown}>
  <button
    type="button"
    class="trigger"
    bind:this={trigger}
    aria-label="Действия с накоплением «{name}»"
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => {
      open = !open;
    }}><span aria-hidden="true">⋯</span></button
  >
  {#if open}
    <ul class="list" role="menu">
      {#each items as item (item.label)}
        <li role="none">
          <button
            type="button"
            role="menuitem"
            class:danger={item.danger}
            disabled={item.disabled}
            onclick={() => {
              pick(item);
            }}>{item.label}</button
          >
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .menu {
    position: relative;
  }
  .trigger {
    width: var(--h-control-sm);
    height: var(--h-control-sm);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-17);
    line-height: 1;
    cursor: pointer;
  }
  .trigger:hover,
  .trigger[aria-expanded='true'] {
    background: var(--line-2);
    color: var(--ink);
  }
  .list {
    position: absolute;
    top: calc(100% + var(--sp-1));
    right: 0;
    z-index: var(--z-sheet);
    display: flex;
    flex-direction: column;
    min-width: 180px;
    margin: 0;
    padding: var(--sp-1);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    box-shadow: var(--shadow);
    list-style: none;
  }
  [role='menuitem'] {
    width: 100%;
    min-height: var(--h-control-sm);
    padding: 0 var(--sp-3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-13);
    text-align: left;
    cursor: pointer;
  }
  [role='menuitem']:hover:enabled,
  [role='menuitem']:focus-visible {
    background: var(--line-2);
  }
  [role='menuitem']:disabled {
    color: var(--muted);
    cursor: not-allowed;
    opacity: 0.6;
  }
  .danger {
    color: var(--unpl);
  }
</style>

<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<kbd>{@render children()}</kbd>

<style>
  kbd {
    display: inline-block;
    padding: 0 6px;
    border: 1px solid var(--line);
    border-radius: 5px;
    background: var(--surface-2);
    color: var(--muted);
    font: var(--fw-medium) var(--fs-11) var(--font-sans);
    line-height: 20px;
    white-space: nowrap;
  }
</style>

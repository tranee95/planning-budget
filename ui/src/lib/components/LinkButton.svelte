<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = { onclick: () => void; children: Snippet };

  let { onclick, children }: Props = $props();
</script>

<button type="button" class="link" {onclick}>{@render children()}</button>

<style>
  .link {
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-xs);
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--fs-13);
    cursor: pointer;
  }
  .link:hover {
    color: var(--ink);
  }
  .link:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

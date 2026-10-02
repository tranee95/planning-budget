<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = { checked?: boolean; disabled?: boolean; children: Snippet };

  let { checked = $bindable(false), disabled = false, children }: Props = $props();
</script>

<label class="check">
  <input type="checkbox" bind:checked {disabled} />
  <span>{@render children()}</span>
</label>

<style>
  .check {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
    min-height: var(--h-control-sm);
    font-size: var(--fs-14);
    cursor: pointer;
  }
  input {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    margin: 2px 0 0;
    accent-color: var(--accent);
    cursor: pointer;
  }
  input:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

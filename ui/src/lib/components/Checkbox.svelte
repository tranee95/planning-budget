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
    margin-top: 2px;
  }
</style>

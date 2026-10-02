<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = {
    checked?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    children: Snippet;
  };

  let { checked = $bindable(false), disabled = false, onchange, children }: Props = $props();
</script>

<label class="switch">
  <input
    type="checkbox"
    role="switch"
    bind:checked
    {disabled}
    onchange={() => onchange?.(checked)}
  />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
  <span>{@render children()}</span>
</label>

<style>
  .switch {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-3);
    min-height: var(--h-control-sm);
    font-size: var(--fs-14);
    cursor: pointer;
  }
  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .track {
    position: relative;
    flex-shrink: 0;
    width: 36px;
    height: 20px;
    border-radius: var(--r-full);
    background: var(--plan-bar);
    transition: background-color var(--dur-base) var(--ease-out);
  }
  .thumb {
    position: absolute;
    inset-block-start: 2px;
    inset-inline-start: 2px;
    width: 16px;
    height: 16px;
    border-radius: var(--r-full);
    background: var(--surface);
    transition: transform var(--dur-base) var(--ease-out);
  }
  input:checked + .track {
    background: var(--accent);
  }
  input:checked + .track .thumb {
    transform: translateX(16px);
  }
  input:focus-visible + .track {
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  input:disabled + .track {
    opacity: 0.45;
  }
</style>

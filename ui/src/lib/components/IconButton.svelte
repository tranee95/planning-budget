<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = {
    'aria-label': string;
    size?: 'sm' | 'md';
    children: Snippet;
  } & HTMLButtonAttributes;

  let { size = 'md', type = 'button', children, ...rest }: Props = $props();
</script>

<button class="icon {size}" {type} {...rest}>{@render children()}</button>

<style>
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition:
      transform var(--dur-fast) var(--ease-out),
      background-color var(--dur-base) var(--ease-out),
      color var(--dur-base) var(--ease-out);
  }
  .md {
    width: var(--h-control);
    height: var(--h-control);
  }
  .sm {
    width: var(--h-control-sm);
    height: var(--h-control-sm);
  }
  .icon:hover:not(:disabled) {
    background: var(--line-2);
    color: var(--ink);
  }
  .icon:active:not(:disabled) {
    transform: scale(0.97);
  }
  .icon:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .icon:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>

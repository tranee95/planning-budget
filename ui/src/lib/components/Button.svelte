<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    loading?: boolean;
    children: Snippet;
  } & HTMLButtonAttributes;

  let {
    variant = 'secondary',
    size = 'md',
    loading = false,
    disabled = false,
    type = 'button',
    children,
    ...rest
  }: Props = $props();
</script>

<button
  class="btn {variant} {size}"
  {type}
  disabled={disabled || loading}
  aria-busy={loading}
  {...rest}
>
  {#if loading}<span class="spinner" aria-hidden="true"></span>{/if}
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    height: var(--h-control);
    padding-inline: var(--sp-4);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fw-medium) var(--fs-14) / 1 var(--font-sans);
    cursor: pointer;
    transition:
      transform var(--dur-fast) var(--ease-out),
      background-color var(--dur-base) var(--ease-out);
  }
  .btn:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .btn:active:not(:disabled) {
    transform: scale(0.97);
  }
  .btn:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .sm {
    height: var(--h-control-sm);
    padding-inline: var(--sp-3);
    font-size: var(--fs-13);
  }
  .lg {
    height: var(--h-control-lg);
    font-size: var(--fs-15);
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .primary:hover:not(:disabled) {
    background: var(--accent);
    filter: brightness(1.06);
  }
  .ghost {
    background: transparent;
    border-color: transparent;
    color: var(--ink-2);
  }
  .ghost:hover:not(:disabled) {
    background: var(--line-2);
  }
  .danger {
    background: var(--unpl-bg);
    border-color: var(--unpl-bar);
    color: var(--unpl);
  }
  .danger:hover:not(:disabled) {
    background: var(--unpl-bg);
    filter: brightness(0.97);
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid currentcolor;
    border-right-color: transparent;
    border-radius: var(--r-full);
    animation: spin calc(var(--dur-progress) * 0.8) linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>

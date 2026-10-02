<script lang="ts">
  import { flip } from 'svelte/animate';
  import X from '@lucide/svelte/icons/x';
  import { dur, toastSlide } from '$lib/motion';
  import { toasts } from '$lib/stores/toasts.svelte';
  import IconButton from './IconButton.svelte';
</script>

<div class="host">
  {#each toasts.items as toast (toast.id)}
    <div
      class="toast {toast.kind}"
      role={toast.kind === 'error' ? 'alert' : 'status'}
      aria-live={toast.kind === 'error' ? 'assertive' : 'polite'}
      transition:toastSlide
      animate:flip={{ duration: dur.base() }}
    >
      <span class="text">
        {toast.message}
        {#if toast.code}<span class="code">{toast.code}</span>{/if}
      </span>
      {#if toast.action}
        <button
          type="button"
          class="action"
          onclick={() => {
            toasts.runAction(toast.id);
          }}
        >
          {toast.action.label}
        </button>
      {/if}
      <IconButton
        size="sm"
        aria-label="Закрыть уведомление"
        onclick={() => {
          toasts.dismiss(toast.id);
        }}
      >
        <X size={16} aria-hidden="true" />
      </IconButton>
    </div>
  {/each}
</div>

<style>
  .host {
    position: fixed;
    inset-block-end: var(--sp-6);
    inset-inline-start: 50%;
    z-index: var(--z-toast);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    transform: translateX(-50%);
  }
  .toast {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    min-width: 280px;
    padding: var(--sp-2) var(--sp-2) var(--sp-2) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--ink);
    color: var(--bg);
    font-size: var(--fs-14);
    box-shadow: var(--shadow);
  }
  .text {
    flex: 1;
  }
  .code {
    margin-inline-start: var(--sp-2);
    font-size: var(--fs-11);
    opacity: 0.7;
  }
  .action {
    height: var(--h-control-sm);
    padding-inline: var(--sp-3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--bg);
    font: var(--fw-semibold) var(--fs-13) var(--font-sans);
    text-decoration: underline;
    cursor: pointer;
  }
  .action:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .toast :global(button) {
    color: var(--bg);
  }
</style>

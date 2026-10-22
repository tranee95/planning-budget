<script lang="ts">
  import { flip } from 'svelte/animate';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import X from '@lucide/svelte/icons/x';
  import { dur, toastSlide } from '$lib/motion';
  import { toasts } from '$lib/stores/toasts.svelte';
  import IconButton from './IconButton.svelte';

  /** Новые сверху. */
  const shown = $derived([...toasts.items].reverse());
</script>

<div class="host">
  {#each shown as toast (toast.id)}
    <div
      class="toast {toast.kind}"
      role={toast.kind === 'error' ? 'alert' : 'status'}
      aria-live={toast.kind === 'error' ? 'assertive' : 'polite'}
      onpointerenter={() => {
        toasts.pause(toast.id);
      }}
      onpointerleave={() => {
        toasts.resume(toast.id);
      }}
      onfocusin={() => {
        toasts.pause(toast.id);
      }}
      onfocusout={() => {
        toasts.resume(toast.id);
      }}
      transition:toastSlide
      animate:flip={{ duration: dur.base() }}
    >
      {#if toast.kind === 'error'}
        <CircleAlert class="icon" size={18} aria-hidden="true" />
      {:else if toast.kind === 'success'}
        <CircleCheck class="icon" size={18} aria-hidden="true" />
      {/if}
      <span class="text">
        {toast.message}
        {#if toast.code}<details class="more">
            <summary>Подробности</summary><span class="code">{toast.code}</span>
          </details>{/if}
      </span>
      {#if toast.ttl !== null}
        {#key toast.rev}<span class="timer" style:--ttl="{toast.ttl}ms" aria-hidden="true"
          ></span>{/key}
      {/if}
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
  /* тонкая полоса внизу тоста: сколько осталось до автозакрытия; под курсором стоит */
  .timer {
    position: absolute;
    inset-inline: 0;
    bottom: 0;
    height: 2px;
    background: currentColor;
    opacity: 0.45;
    transform-origin: left;
    animation: timer var(--ttl) linear forwards;
  }
  .toast:hover .timer,
  .toast:focus-within .timer {
    animation-play-state: paused;
  }
  :global(:root[data-motion='reduce']) .timer {
    display: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .timer {
      display: none;
    }
  }
  @keyframes timer {
    to {
      transform: scaleX(0);
    }
  }
  .toast {
    position: relative;
    overflow: hidden;
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
  /* Акцент слева: цвет значения, чтобы ошибку и успех отличал не только текст (WCAG 1.4.1) */
  .toast.error {
    box-shadow:
      inset 4px 0 0 var(--unpl-bar),
      var(--shadow);
  }
  .toast.success {
    box-shadow:
      inset 4px 0 0 var(--paid-bar),
      var(--shadow);
  }
  .toast :global(.icon) {
    flex: none;
  }
  .toast.error :global(.icon) {
    color: var(--unpl-bar);
  }
  .toast.success :global(.icon) {
    color: var(--paid-bar);
  }
  .text {
    flex: 1;
  }
  .more {
    margin-block-start: var(--sp-1);
    font-size: var(--fs-12);
    opacity: 0.8;
  }
  .more summary {
    cursor: pointer;
  }
  .code {
    margin-inline-start: 0;
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
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
  }
  .toast :global(button) {
    color: var(--bg);
  }
</style>

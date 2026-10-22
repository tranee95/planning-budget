<script lang="ts">
  import type { Snippet } from 'svelte';
  import { pop, scrimFade } from '$lib/motion';
  import X from '@lucide/svelte/icons/x';
  import IconButton from './IconButton.svelte';
  import { trapFocus } from './overlay';

  type Props = {
    title: string;
    width?: number;
    onclose: () => void;
    children: Snippet;
    /** Кнопки действий: закреплены внизу и видны при любой высоте окна. */
    footer?: Snippet;
    /** Крестик закрытия в шапке (для форм, где нужен явный выход, кроме Esc и клика по фону). */
    closable?: boolean;
  };

  let { title, width = 560, onclose, children, footer, closable = false }: Props = $props();

  const titleId = $props.id();
</script>

<div class="layer">
  <button
    type="button"
    class="scrim"
    tabindex="-1"
    aria-label="Закрыть"
    onclick={onclose}
    transition:scrimFade
  ></button>
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    style:max-width="{width}px"
    use:trapFocus={onclose}
    transition:pop
  >
    {#if closable}
      <div class="head">
        <h2 id={titleId}>{title}</h2>
        <IconButton size="sm" aria-label="Закрыть" onclick={onclose}>
          <X size={18} aria-hidden="true" />
        </IconButton>
      </div>
    {:else}
      <h2 id={titleId}>{title}</h2>
    {/if}
    <div class="content">{@render children()}</div>
    {#if footer}<div class="footer">{@render footer()}</div>{/if}
  </div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    display: grid;
    place-items: center;
    /* Строка с определённой высотой: иначе max-height панели в процентах не работает. */
    grid-template-rows: minmax(0, 1fr);
    padding: var(--sp-6);
  }
  .scrim {
    position: absolute;
    inset: 0;
    border: 0;
    background: var(--scrim);
    /* размытие фона: затемнение в тёмной теме и так почти не видно */
    backdrop-filter: blur(4px);
    cursor: default;
  }
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    width: 100%;
    max-height: 100%;
    padding: var(--sp-6);
    border-radius: var(--r-xl);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  /* Отступ с компенсирующим margin: кольцо фокуса (3 px) у краёв области не обрезается. */
  .content {
    display: flex;
    padding: var(--sp-1);
    margin: calc(-1 * var(--sp-1));
    flex-direction: column;
    gap: var(--sp-4);
    min-height: 0;
    overflow-y: auto;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
</style>

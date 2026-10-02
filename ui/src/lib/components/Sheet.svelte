<script lang="ts">
  import type { Snippet } from 'svelte';
  import X from '@lucide/svelte/icons/x';
  import { scrimFade, sheetSlide } from '$lib/motion';
  import IconButton from './IconButton.svelte';
  import { trapFocus } from './overlay';

  type Props = {
    title: string;
    onclose: () => void;
    children: Snippet;
    /** Ширина панели в px; по умолчанию 420. */
    width?: number;
    /** Кнопки действий: закреплены внизу и видны при любой высоте окна. */
    footer?: Snippet;
  };

  let { title, onclose, children, width = 420, footer }: Props = $props();

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
    style:width={`min(${String(width)}px, 100%)`}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    use:trapFocus={onclose}
    transition:sheetSlide
  >
    <header>
      <h2 id={titleId}>{title}</h2>
      <IconButton size="sm" aria-label="Закрыть панель" onclick={onclose}>
        <X size={18} aria-hidden="true" />
      </IconButton>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<div class="footer">{@render footer()}</div>{/if}
  </div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: var(--z-sheet);
  }
  .scrim {
    position: absolute;
    inset: 0;
    border: 0;
    background: var(--scrim);
    cursor: default;
  }
  .panel {
    position: absolute;
    inset-block: 0;
    inset-inline-end: 0;
    display: flex;
    flex-direction: column;
    border-inline-start: 1px solid var(--line);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-4) var(--sp-5);
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  .body {
    flex: 1;
    padding: var(--sp-5);
    overflow-y: auto;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
    padding: var(--sp-4) var(--sp-5);
    border-top: 1px solid var(--line);
  }
</style>

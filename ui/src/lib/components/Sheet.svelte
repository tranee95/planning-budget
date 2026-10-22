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

  /** Под футером есть непрокрученное содержимое: над ним показывается затухание. */
  let moreBelow = $state(false);

  function measure(node: HTMLElement): void {
    moreBelow = node.scrollHeight - node.scrollTop - node.clientHeight > 1;
  }

  function watch(node: HTMLElement) {
    measure(node);
    const observer = new ResizeObserver(() => {
      measure(node);
    });
    observer.observe(node);
    for (const child of node.children) observer.observe(child);
    return {
      destroy: () => {
        observer.disconnect();
      }
    };
  }
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
    <div
      class="body"
      use:watch
      onscroll={(event) => {
        measure(event.currentTarget);
      }}
    >
      {@render children()}
    </div>
    {#if footer}<div class="footer" class:fade={moreBelow}>{@render footer()}</div>{/if}
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
    /* размытие фона: затемнение в тёмной теме и так почти не видно */
    backdrop-filter: blur(4px);
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
    /* запас под футер: последняя строка не прижата к его границе */
    padding-block-end: var(--sp-6);
    overflow-y: auto;
  }
  .footer {
    position: relative;
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
    padding: var(--sp-4) var(--sp-5);
    border-top: 1px solid var(--line);
  }
  .footer::before {
    content: '';
    position: absolute;
    inset-inline: 0;
    bottom: 100%;
    height: 24px;
    background: linear-gradient(to bottom, transparent, var(--surface));
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--dur-fast) var(--ease-out);
  }
  .footer.fade::before {
    opacity: 1;
  }
</style>

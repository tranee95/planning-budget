<script lang="ts">
  import type { Snippet } from 'svelte';
  import { clampShift } from './popover';

  type Props = { text: string; shortcut?: string; children: Snippet };

  let { text, shortcut, children }: Props = $props();

  const id = $props.id();
  let wrap = $state<HTMLSpanElement>();
  let tip = $state<HTMLSpanElement>();
  let shift = $state(0);

  // У края окна подсказка сдвигается внутрь, а не обрезается.
  function fit(): void {
    if (!wrap || !tip) return;
    const zoom = parseFloat(document.documentElement.style.zoom) || 1;
    const w = wrap.getBoundingClientRect();
    shift =
      clampShift({
        center: w.left + w.width / 2,
        width: tip.getBoundingClientRect().width,
        viewport: document.documentElement.clientWidth
      }) / zoom;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="wrap" aria-describedby={id} bind:this={wrap} onpointerenter={fit} onfocusin={fit}>
  {@render children()}
  <span class="tip" role="tooltip" {id} bind:this={tip} style:--shift="{shift}px">
    {text}{#if shortcut}<kbd>{shortcut}</kbd>{/if}
  </span>
</span>

<style>
  .wrap {
    position: relative;
    display: inline-flex;
  }
  .tip {
    position: absolute;
    inset-block-start: calc(100% + var(--sp-1));
    inset-inline-start: 50%;
    z-index: var(--z-tooltip);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-2);
    border-radius: var(--r-xs);
    background: var(--ink);
    color: var(--bg);
    font-size: var(--fs-12);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transform: translateX(calc(-50% + var(--shift, 0px)));
    transition: opacity var(--dur-fast) var(--ease-out);
  }
  .wrap:hover .tip,
  .wrap:focus-within .tip {
    opacity: 1;
  }
  kbd {
    font: var(--fw-medium) var(--fs-11) var(--font-sans);
    opacity: 0.7;
  }
</style>

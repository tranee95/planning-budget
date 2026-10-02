<script lang="ts">
  import { onMount } from 'svelte';
  import { flip } from '$lib/charts/flip';
  import { COLUMNS, ROW_HEIGHT, type Box, type SizeName } from '$lib/charts/layout';
  import { categories } from '$lib/stores/categories.svelte';
  import { getAnalyticsVm } from '../analytics.svelte';
  import { getBuilderVm } from '../builder.svelte';
  import ChartCard from './ChartCard.svelte';

  type Props = { ondrill: (query: string) => void };

  let { ondrill }: Props = $props();

  const vm = getAnalyticsVm();
  const builder = getBuilderVm();

  /** Расстояние между карточками: токен `--sp-4`. */
  const GAP = 16;
  /** Полоса у края окна, в которой перетаскивание прокручивает страницу. */
  const EDGE = 64;

  type Drag = { id: number; pointerX: number; pointerY: number; fromX: number; fromY: number };

  let grid = $state<HTMLDivElement>();
  let drag = $state<Drag | null>(null);
  let shift = $state({ x: 0, y: 0 });

  const categoryColors = $derived(new Map(categories.items.map((c) => [c.id, c.color])));
  const boxes = $derived(new Map(vm.layout.map((b) => [b.id, b])));
  const ordered = $derived(
    [...vm.cards].sort((a, b) => {
      const pa = boxes.get(a.id);
      const pb = boxes.get(b.id);
      return (pa?.y ?? 0) - (pb?.y ?? 0) || (pa?.x ?? 0) - (pb?.x ?? 0) || a.id - b.id;
    })
  );

  function pitch(): { x: number; y: number } {
    const columnWidth = ((grid?.clientWidth ?? 0) - GAP * (COLUMNS - 1)) / COLUMNS;
    return { x: columnWidth + GAP, y: ROW_HEIGHT + GAP };
  }

  function scrollParent(node: HTMLElement | undefined): HTMLElement | null {
    for (let el = node?.parentElement ?? null; el !== null; el = el.parentElement) {
      if (/(auto|scroll)/.test(getComputedStyle(el).overflowY)) return el;
    }
    return null;
  }

  function start(event: PointerEvent, box: Box): void {
    if (event.button !== 0) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    drag = {
      id: box.id,
      pointerX: event.clientX,
      pointerY: event.clientY,
      fromX: box.x,
      fromY: box.y
    };
    shift = { x: 0, y: 0 };
  }

  function target(current: Drag, event: PointerEvent): Partial<Box> {
    const step = pitch();
    return {
      x: Math.round(current.fromX + (event.clientX - current.pointerX) / step.x),
      y: Math.round(current.fromY + (event.clientY - current.pointerY) / step.y)
    };
  }

  function move(event: PointerEvent): void {
    if (drag === null) return;
    const to = target(drag, event);
    vm.preview(drag.id, to);
    const step = pitch();
    const placed = boxes.get(drag.id);
    shift = {
      x: event.clientX - drag.pointerX - ((placed?.x ?? drag.fromX) - drag.fromX) * step.x,
      y: event.clientY - drag.pointerY - ((placed?.y ?? drag.fromY) - drag.fromY) * step.y
    };
    const scroller = scrollParent(grid);
    if (scroller !== null) {
      const rect = scroller.getBoundingClientRect();
      if (event.clientY < rect.top + EDGE) scroller.scrollBy({ top: -20 });
      else if (event.clientY > rect.bottom - EDGE) scroller.scrollBy({ top: 20 });
    }
  }

  function finish(event: PointerEvent): void {
    if (drag === null) return;
    const done = drag;
    drag = null;
    shift = { x: 0, y: 0 };
    void vm.commit(done.id, target(done, event));
  }

  function cancel(): void {
    if (drag === null) return;
    drag = null;
    shift = { x: 0, y: 0 };
    vm.cancelPreview();
  }

  function keyboard(event: KeyboardEvent, box: Box): void {
    const delta: Record<string, Partial<Box>> = {
      ArrowLeft: { x: box.x - 1 },
      ArrowRight: { x: box.x + 1 },
      ArrowUp: { y: box.y - 1 },
      ArrowDown: { y: box.y + 1 }
    };
    const to = delta[event.key];
    if (to === undefined) return;
    event.preventDefault();
    void vm.commit(box.id, to);
  }

  onMount(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
    };
  });
</script>

<div
  class="grid"
  bind:this={grid}
  onpointermove={move}
  onpointerup={finish}
  onpointercancel={cancel}
  role="presentation"
>
  {#each ordered as card (card.id)}
    {@const box = boxes.get(card.id) ?? { id: card.id, x: card.x, y: card.y, w: card.w, h: card.h }}
    {@const dragging = drag?.id === card.id}
    <div
      class="cell"
      class:dragging
      style:grid-column={`${String(box.x + 1)} / span ${String(box.w)}`}
      style:grid-row={`${String(box.y + 1)} / span ${String(box.h)}`}
      style:transform={dragging
        ? `translate(${String(shift.x)}px, ${String(shift.y)}px)`
        : undefined}
      use:flip={{ shiftX: dragging ? shift.x : 0, shiftY: dragging ? shift.y : 0, dragging }}
    >
      <ChartCard
        {card}
        {box}
        progress={vm.data.get(card.id)}
        {categoryColors}
        {dragging}
        onhandlepointerdown={(event: PointerEvent) => {
          start(event, box);
        }}
        onhandlekeydown={(event: KeyboardEvent) => {
          keyboard(event, box);
        }}
        onedit={() => {
          builder.beginEdit(card.id, card.spec);
        }}
        onresize={(size: SizeName) => {
          void vm.resize(card.id, size);
        }}
        onduplicate={() => {
          void vm.duplicate(card.id);
        }}
        onremove={() => {
          void vm.remove(card.id);
        }}
        {ondrill}
      />
    </div>
  {/each}
</div>

<style>
  .grid {
    position: relative;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    grid-auto-rows: 80px;
    gap: var(--sp-4);
  }
  .cell {
    min-width: 0;
    min-height: 0;
  }
  .cell.dragging {
    position: relative;
    z-index: 5;
  }
</style>

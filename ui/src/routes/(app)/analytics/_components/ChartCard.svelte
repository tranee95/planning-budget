<script lang="ts">
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import Copy from '@lucide/svelte/icons/copy';
  import GripVertical from '@lucide/svelte/icons/grip-vertical';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import type { ChartCardDto } from '$lib/api/bindings';
  import type { CardData } from '$lib/charts/card-data';
  import type { CategoryColors } from '$lib/charts/colors';
  import { SIZES, sizeOf, type Box, type SizeName } from '$lib/charts/layout';
  import { Chart, EmptyState, IconButton, Skeleton } from '$lib/components';

  type Props = {
    card: ChartCardDto;
    box: Box;
    progress: CardData | undefined;
    categoryColors: CategoryColors;
    dragging: boolean;
    onhandlepointerdown: (event: PointerEvent) => void;
    onhandlekeydown: (event: KeyboardEvent) => void;
    onedit: () => void;
    onresize: (size: SizeName) => void;
    onduplicate: () => void;
    onremove: () => void;
    ondrill: (query: string) => void;
  };

  let {
    card,
    box,
    progress,
    categoryColors,
    dragging,
    onhandlepointerdown,
    onhandlekeydown,
    onedit,
    onresize,
    onduplicate,
    onremove,
    ondrill
  }: Props = $props();

  const current = $derived(sizeOf(box));
  const ready = $derived(progress?.status === 'ready' ? progress.data : null);
  const failure = $derived(progress?.status === 'error' ? progress.message : null);
</script>

<section class="card" class:dragging aria-label={card.spec.title}>
  <header>
    <button
      type="button"
      class="grip"
      aria-label={`Переместить «${card.spec.title}»: перетащите или нажимайте стрелки`}
      onpointerdown={onhandlepointerdown}
      onkeydown={onhandlekeydown}
    >
      <GripVertical size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <h2 title={card.spec.title}>{card.spec.title}</h2>
    <div class="tools">
      <div class="sizes" role="group" aria-label={`Размер «${card.spec.title}»`}>
        {#each SIZES as size (size.value)}
          <button
            type="button"
            class:on={current === size.value}
            aria-pressed={current === size.value}
            onclick={() => {
              onresize(size.value);
            }}>{size.label}</button
          >
        {/each}
      </div>
      <IconButton size="sm" aria-label={`Настроить «${card.spec.title}»`} onclick={onedit}>
        <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
      </IconButton>
      <IconButton size="sm" aria-label={`Дублировать «${card.spec.title}»`} onclick={onduplicate}>
        <Copy size={16} strokeWidth={1.75} aria-hidden="true" />
      </IconButton>
      <IconButton size="sm" aria-label={`Удалить «${card.spec.title}»`} onclick={onremove}>
        <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
      </IconButton>
    </div>
  </header>
  <div class="body">
    {#if failure !== null}
      <EmptyState title="Не удалось построить график" hint={failure} />
    {:else if ready === null}
      <Skeleton height="100%" />
    {:else if ready.categories.length === 0}
      <EmptyState title="Нет данных за период" hint="Измените период или фильтр графика." />
    {:else}
      <Chart
        data={ready}
        type={card.spec.type}
        title={card.spec.title}
        options={card.spec.options}
        spec={card.spec}
        {categoryColors}
        {ondrill}
      />
    {/if}
  </div>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    height: 100%;
    min-width: 0;
    box-sizing: border-box;
    padding: var(--sp-4) var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .dragging {
    border-color: var(--accent);
    box-shadow: var(--shadow);
    cursor: grabbing;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-1) var(--sp-2);
    min-width: 0;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin-left: auto;
  }
  h2 {
    flex: 1 1 180px;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow-wrap: anywhere;
    font-size: var(--fs-14);
    font-weight: var(--fw-semibold);
  }
  .grip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--h-control-sm);
    height: var(--h-control-sm);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--faint);
    cursor: grab;
    touch-action: none;
  }
  .grip:hover {
    background: var(--line-2);
    color: var(--ink-2);
  }
  .sizes {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: var(--r-sm);
    background: var(--line-2);
  }
  .sizes button {
    min-width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--r-xs);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
    cursor: pointer;
  }
  .sizes button.on {
    background: var(--surface);
    color: var(--ink);
    box-shadow: var(--shadow);
  }
  .body {
    flex: 1;
    min-height: 0;
  }
</style>

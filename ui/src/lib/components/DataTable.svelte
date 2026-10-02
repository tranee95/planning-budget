<script lang="ts" generics="Row">
  import type { Snippet } from 'svelte';
  import { clampActive, visibleRange } from './virtual';
  import type { Column } from './table';

  type Props = {
    columns: readonly Column[];
    rows: readonly Row[];
    rowKey: (row: Row) => number | string;
    cell: Snippet<[row: Row, column: Column]>;
    label: string;
    rowHeight?: number;
    sort?: { column: string; descending: boolean } | null;
    onsort?: (column: string) => void;
    selected?: ReadonlySet<number | string>;
    onselect?: (key: number | string) => void;
    onopen?: (row: Row) => void;
    empty?: Snippet;
  };

  let {
    columns,
    rows,
    rowKey,
    cell,
    label,
    rowHeight = 44,
    sort = null,
    onsort,
    selected,
    onselect,
    onopen,
    empty
  }: Props = $props();

  const OVERSCAN = 6;
  const uid = $props.id();

  let scrollTop = $state(0);
  let viewport = $state(480);
  let active = $state(0);
  let body: HTMLDivElement | undefined = $state();

  const template = $derived(columns.map((c) => c.width ?? '1fr').join(' '));
  const range = $derived(
    visibleRange({ scrollTop, viewport, rowHeight, count: rows.length, overscan: OVERSCAN })
  );
  const slice = $derived(rows.slice(range.start, range.end));
  const activeIndex = $derived(Math.min(active, Math.max(rows.length - 1, 0)));

  // Пустой список размонтирует прокручиваемый блок: новый начнётся с нуля.
  $effect(() => {
    if (rows.length === 0) {
      scrollTop = 0;
      active = 0;
    }
  });

  function reveal(index: number): void {
    if (!body) return;
    const top = index * rowHeight;
    if (top < body.scrollTop) body.scrollTop = top;
    else if (top + rowHeight > body.scrollTop + viewport) {
      body.scrollTop = top + rowHeight - viewport;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    // Клавиши из кнопок и полей внутри ячеек принадлежат им, а не таблице.
    if (event.target !== event.currentTarget) return;
    const last = rows.length - 1;
    const jump: Record<string, number | undefined> = {
      ArrowDown: activeIndex + 1,
      ArrowUp: activeIndex - 1,
      PageDown: activeIndex + Math.floor(viewport / rowHeight),
      PageUp: activeIndex - Math.floor(viewport / rowHeight),
      Home: 0,
      End: last
    };
    const target = jump[event.key];
    if (target !== undefined) {
      event.preventDefault();
      active = Math.min(Math.max(target, 0), Math.max(last, 0));
      reveal(active);
      return;
    }
    const row = rows[activeIndex];
    if (!row) return;
    if (event.key === ' ' && onselect) {
      event.preventDefault();
      onselect(rowKey(row));
    } else if (event.key === 'Enter' && onopen) {
      event.preventDefault();
      onopen(row);
    }
  }

  const ariaSort = (column: Column): 'ascending' | 'descending' | 'none' | undefined => {
    if (!column.sortable) return undefined;
    if (sort?.column !== column.id) return 'none';
    return sort.descending ? 'descending' : 'ascending';
  };
</script>

<!-- Фокус держит сам грид, активную строку для скринридера задаёт aria-activedescendant. -->
<div
  class="table"
  role="grid"
  tabindex="0"
  aria-label={label}
  aria-rowcount={rows.length + 1}
  aria-activedescendant={rows.length > 0 ? `${uid}-row-${String(activeIndex)}` : undefined}
  onkeydown={onKeydown}
>
  <div class="head" role="row" style:grid-template-columns={template}>
    {#each columns as column (column.id)}
      <div
        class="th"
        role="columnheader"
        aria-sort={ariaSort(column)}
        class:end={column.align === 'end'}
      >
        {#if column.sortable && onsort}
          <button
            type="button"
            onclick={() => {
              onsort(column.id);
            }}
          >
            {column.label}
            {#if sort?.column === column.id}<span aria-hidden="true"
                >{sort.descending ? '↓' : '↑'}</span
              >{/if}
          </button>
        {:else}
          {column.label}
        {/if}
      </div>
    {/each}
  </div>

  {#if rows.length === 0}
    {@render empty?.()}
  {:else}
    <div
      class="body"
      bind:this={body}
      bind:clientHeight={viewport}
      role="rowgroup"
      onscroll={(event) => {
        scrollTop = event.currentTarget.scrollTop;
        // Колесо мыши уводит активную строку из окна: переносим её на видимую (aria-activedescendant).
        active = clampActive({ scrollTop, viewport, rowHeight, count: rows.length, active });
      }}
    >
      <div class="spacer" role="presentation" style:height="{rows.length * rowHeight}px">
        {#each slice as row, i (rowKey(row))}
          {@const index = range.start + i}
          {@const key = rowKey(row)}
          <div
            id="{uid}-row-{index}"
            class="row"
            role="row"
            aria-rowindex={index + 2}
            aria-selected={selected ? selected.has(key) : undefined}
            class:active={index === activeIndex}
            class:selected={selected?.has(key)}
            style:grid-template-columns={template}
            style:height="{rowHeight}px"
            style:transform="translateY({index * rowHeight}px)"
          >
            {#each columns as column (column.id)}
              <div class="td" role="gridcell" class:end={column.align === 'end'}>
                {@render cell(row, column)}
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .table {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    overflow: hidden;
  }
  .head,
  .row {
    display: grid;
    align-items: center;
    column-gap: var(--sp-3);
    padding-inline: var(--sp-4);
  }
  .head {
    height: var(--h-control);
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .th button {
    min-height: 28px;
    padding: 0 var(--sp-1);
    /* Отступ кнопки нужен кольцу фокуса, но текст заголовка должен стоять над текстом ячеек. */
    margin-inline: calc(var(--sp-1) * -1);
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .th button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .table:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 3px var(--accent-soft);
  }
  .spacer {
    position: relative;
  }
  .row {
    position: absolute;
    inset-inline: 0;
    border-bottom: 1px solid var(--line-2);
    font-size: var(--fs-14);
  }
  .row.selected {
    background: var(--accent-soft);
  }
  .table:focus-visible .row.active {
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .td {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .end {
    text-align: end;
    font-variant-numeric: tabular-nums;
  }
</style>

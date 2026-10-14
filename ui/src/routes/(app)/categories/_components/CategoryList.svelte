<script lang="ts">
  import Archive from '@lucide/svelte/icons/archive';
  import ArchiveRestore from '@lucide/svelte/icons/archive-restore';
  import GripVertical from '@lucide/svelte/icons/grip-vertical';
  import type { CategoryKindDto } from '$lib/api/bindings';
  import { IconButton } from '$lib/components';
  import { formatMoney } from '$lib/format';
  import { getCategoriesVm } from '../categories.svelte';
  import { rowIndexAt } from '../reorder';

  const vm = getCategoriesVm();

  const KIND_LABEL: Record<CategoryKindDto, string> = {
    mandatory: 'Обязательные',
    wants: 'Желания',
    savings: 'Сбережения',
    loans: 'Займы'
  };

  let list = $state<HTMLUListElement>();
  let dragId = $state<number | null>(null);
  let overId = $state<number | null>(null);

  /**
   * Перетаскивание на pointer events: HTML5 drag-and-drop в WebView2 перехватывается оболочкой
   * окна, поэтому строки не переставлялись. Ручка захватывает указатель, строка под ним
   * подсвечивается, отпускание переносит категорию, Escape отменяет.
   */
  function overRow(clientY: number): number | null {
    if (!list) return null;
    const bounds = [...list.children].map((li) => li.getBoundingClientRect());
    const index = rowIndexAt(clientY, bounds);
    return index === null ? null : (vm.rows[index]?.category.id ?? null);
  }

  function startDrag(event: PointerEvent, id: number): void {
    if (event.button !== 0) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragId = id;
    overId = id;
  }

  function moveDrag(event: PointerEvent): void {
    if (dragId === null) return;
    overId = overRow(event.clientY);
  }

  function endDrag(event: PointerEvent): void {
    if (dragId === null) return;
    const from = dragId;
    const to = overRow(event.clientY);
    dragId = null;
    overId = null;
    if (to !== null && to !== from) void vm.move(from, to);
  }

  function cancelDrag(): void {
    dragId = null;
    overId = null;
  }

  function onKeydown(event: KeyboardEvent, id: number): void {
    if (event.key === 'Enter' && event.target === event.currentTarget) vm.select(id);
  }
</script>

<svelte:window
  onkeydown={(e: KeyboardEvent) => {
    if (e.key === 'Escape' && dragId !== null) cancelDrag();
  }}
/>

<div class="head" aria-hidden="true">
  <span></span><span></span><span>Категория</span><span>Тип</span>
  <span class="r">Лимит</span><span class="r">Среднее за год</span><span class="r"
    >Превышение лимита</span
  >
</div>
<ul role="list" bind:this={list} class:dragging={dragId !== null}>
  {#each vm.rows as { category, limit, avg, overLimit } (category.id)}
    <li
      class="item"
      class:active={vm.selected === category.id}
      class:archived={category.archived}
      class:lifted={dragId === category.id}
      class:target={dragId !== null && overId === category.id && dragId !== category.id}
    >
      <button
        type="button"
        class="grip"
        aria-label="Перетащить"
        aria-describedby="cat-name-{category.id}"
        title="Перетащить, чтобы изменить порядок"
        onpointerdown={(e) => {
          startDrag(e, category.id);
        }}
        onpointermove={moveDrag}
        onpointerup={endDrag}
        onpointercancel={cancelDrag}
      >
        <GripVertical size={16} />
      </button>
      <div
        class="row"
        role="button"
        tabindex="0"
        aria-label="{category.name}, {KIND_LABEL[category.kind]}"
        onclick={() => {
          vm.select(category.id);
        }}
        onkeydown={(e) => {
          onKeydown(e, category.id);
        }}
      >
        <span class="swatch" style:background={category.color} aria-hidden="true"></span>
        <span class="name" id="cat-name-{category.id}"
          >{category.name}{category.archived ? ' · архив' : ''}</span
        >
        <span class="kind">{KIND_LABEL[category.kind]}</span>
        <span class="num r">{limit === null ? '—' : formatMoney(limit)}</span>
        <span class="num r muted">{avg === 0 ? '—' : formatMoney(avg)}</span>
        <span class="r">
          {#if overLimit !== null && overLimit > 0}
            <span class="num over">+{formatMoney(overLimit)}</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        </span>
      </div>
      <span class="actions">
        {#if category.archived}
          <IconButton
            size="sm"
            aria-label="Вернуть из архива"
            aria-describedby="cat-name-{category.id}"
            title="Вернуть из архива"
            onclick={() => {
              void vm.setArchived(category.id, false);
            }}><ArchiveRestore size={16} /></IconButton
          >
        {:else}
          <IconButton
            size="sm"
            aria-label="В архив"
            aria-describedby="cat-name-{category.id}"
            title="В архив"
            onclick={() => {
              vm.askArchive(category.id);
            }}><Archive size={16} /></IconButton
          >
        {/if}
      </span>
    </li>
  {/each}
</ul>

<style>
  /* Колонки заголовка и строки совпадают: ручка 24 px, затем цвет, название, тип и числа. */
  .head,
  .row {
    display: grid;
    align-items: center;
    gap: var(--sp-3);
  }
  .head {
    grid-template-columns: 24px 14px minmax(0, 1fr) 120px 130px 110px 140px;
    padding-block: var(--sp-2);
    /* Справа — столбец кнопки архива (32 px) и отступы строки. */
    padding-inline: var(--sp-4) calc(var(--sp-4) + var(--sp-3) + 32px + var(--sp-2));
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .row {
    grid-template-columns: 14px minmax(0, 1fr) 120px 130px 110px 140px;
    padding-inline-end: var(--sp-4);
  }
  .item {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr) 32px;
    align-items: center;
    gap: var(--sp-3);
    padding-inline: var(--sp-4) var(--sp-2);
    border-radius: var(--r-md);
    transition: background var(--dur-base) var(--ease-out);
  }
  .item:hover {
    background: var(--surface-2);
  }
  .item.active {
    background: var(--accent-soft);
  }
  .item.archived .row {
    opacity: 0.6;
  }
  .item.lifted {
    background: var(--surface-2);
    opacity: 0.7;
  }
  .item.target {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .actions {
    display: inline-flex;
    justify-content: center;
  }
  /* Список делит экран с редактором, поэтому колонки отключаются по ширине самого списка:
     сначала «Среднее за год» и «Превышение лимита», затем «Тип» (он виден в редакторе выбранной строки). */
  @container (max-width: 790px) {
    .head {
      grid-template-columns: 24px 14px minmax(0, 1fr) 120px 130px;
    }
    .row {
      grid-template-columns: 14px minmax(0, 1fr) 120px 130px;
    }
    .head > :nth-child(n + 6),
    .row > :nth-child(n + 5) {
      display: none;
    }
  }
  @container (max-width: 520px) {
    .head {
      grid-template-columns: 24px 14px minmax(0, 1fr) 130px;
    }
    .row {
      grid-template-columns: 14px minmax(0, 1fr) 130px;
    }
    .head > :nth-child(4),
    .row > :nth-child(3) {
      display: none;
    }
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  ul.dragging,
  ul.dragging * {
    cursor: grabbing;
    user-select: none;
  }
  .row {
    height: 48px;
    border-radius: var(--r-md);
    font-size: var(--fs-14);
    cursor: pointer;
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .grip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--faint);
    cursor: grab;
    touch-action: none;
  }
  .grip:hover {
    color: var(--ink-2);
  }
  .grip:focus-visible {
    outline: 2px solid var(--accent);
  }
  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 4px;
  }
  .name {
    overflow: hidden;
    font-weight: var(--fw-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    justify-self: start;
    height: 22px;
    padding-inline: var(--sp-2);
    border-radius: var(--r-full);
    background: var(--line-2);
    color: var(--ink-2);
    font-size: var(--fs-11);
    line-height: 22px;
  }
  .r {
    text-align: right;
  }
  .muted {
    color: var(--muted);
  }
  .over {
    color: var(--unpl);
    font-weight: var(--fw-semibold);
  }
</style>

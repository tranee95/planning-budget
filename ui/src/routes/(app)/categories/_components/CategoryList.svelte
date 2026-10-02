<script lang="ts">
  import GripVertical from '@lucide/svelte/icons/grip-vertical';
  import type { CategoryKindDto } from '$lib/api/bindings';
  import { formatMoney } from '$lib/format';
  import { formatPercent } from '$lib/format';
  import { getCategoriesVm } from '../categories.svelte';

  const vm = getCategoriesVm();

  const KIND_LABEL: Record<CategoryKindDto, string> = {
    mandatory: 'Обязательные',
    wants: 'Желания',
    savings: 'Сбережения',
    loans: 'Займы'
  };

  let dragId = $state<number | null>(null);
  let overId = $state<number | null>(null);

  function onKeydown(event: KeyboardEvent, id: number): void {
    if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
      event.preventDefault();
      void vm.nudge(id, event.key === 'ArrowUp' ? -1 : 1);
    } else if (event.key === 'Enter' && event.target === event.currentTarget) {
      vm.select(id);
    }
  }
</script>

<div class="head" aria-hidden="true">
  <span></span><span></span><span>Категория</span><span>Тип</span>
  <span class="r">Лимит</span><span class="r">Среднее</span><span class="r">Выше лимита</span>
</div>
<ul role="list">
  {#each vm.rows as { category, limit, rateBp, avg, overLimit } (category.id)}
    <li>
      <div
        class="row"
        role="button"
        class:active={vm.selected === category.id}
        class:archived={category.archived}
        class:target={overId === category.id && dragId !== category.id}
        draggable="true"
        tabindex="0"
        aria-label="{category.name}, {KIND_LABEL[category.kind]}"
        aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
        ondragstart={(e) => {
          dragId = category.id;
          if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
        }}
        ondragover={(e) => {
          e.preventDefault();
          overId = category.id;
        }}
        ondragend={() => {
          dragId = null;
          overId = null;
        }}
        ondrop={(e) => {
          e.preventDefault();
          if (dragId !== null) void vm.move(dragId, category.id);
          dragId = null;
          overId = null;
        }}
        onclick={() => {
          vm.select(category.id);
        }}
        onkeydown={(e) => {
          onKeydown(e, category.id);
        }}
      >
        <span class="grip" aria-hidden="true"><GripVertical size={16} /></span>
        <span class="swatch" style:background={category.color} aria-hidden="true"></span>
        <span class="name">{category.name}{category.archived ? ' · архив' : ''}</span>
        <span class="kind">{KIND_LABEL[category.kind]}</span>
        <span class="num r">
          {#if category.kind === 'savings'}
            {rateBp === null ? '—' : `${formatPercent(rateBp)} дохода`}
          {:else}
            {limit === null ? '—' : formatMoney(limit)}
          {/if}
        </span>
        <span class="num r muted">{avg === 0 ? '—' : formatMoney(avg)}</span>
        <span class="r">
          {#if overLimit !== null && overLimit > 0}
            <span class="num over">+{formatMoney(overLimit)}</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        </span>
      </div>
    </li>
  {/each}
</ul>

<style>
  .head,
  .row {
    display: grid;
    grid-template-columns: 24px 14px minmax(0, 1fr) 120px 130px 110px 110px;
    align-items: center;
    gap: var(--sp-3);
    padding-inline: 14px;
  }
  .head {
    padding-block: 6px;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  /* Список делит экран с редактором, поэтому колонки отключаются по ширине самого списка:
     сначала «Среднее» и «Выше лимита», затем «Тип» (он виден в редакторе выбранной строки). */
  @container (max-width: 760px) {
    .head,
    .row {
      grid-template-columns: 24px 14px minmax(0, 1fr) 120px 130px;
    }
    .head > :nth-child(n + 6),
    .row > :nth-child(n + 6) {
      display: none;
    }
  }
  @container (max-width: 520px) {
    .head,
    .row {
      grid-template-columns: 24px 14px minmax(0, 1fr) 130px;
    }
    .head > :nth-child(4),
    .row > :nth-child(4) {
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
  .row {
    height: 48px;
    border-radius: var(--r-md);
    font-size: var(--fs-14);
    cursor: pointer;
    transition: background var(--dur-base) var(--ease-out);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.active {
    background: var(--accent-soft);
  }
  .row.archived {
    opacity: 0.6;
  }
  .row.target {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .grip {
    display: inline-flex;
    color: var(--faint);
    cursor: grab;
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

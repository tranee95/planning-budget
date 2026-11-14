<script lang="ts">
  import { deleteOnKey } from '$lib/components/row-delete';
  import { rowIn, rowOut } from '$lib/motion';
  import type { TransactionDto } from '$lib/api/bindings';
  import { StatusDot } from '$lib/components';
  import { STATUS_LABEL } from '$lib/components/status';
  import { formatMoney } from '$lib/format';
  import InlineEdit from '$lib/components/amount-rows/InlineEdit.svelte';

  type Props = {
    row: TransactionDto;
    editing: boolean;
    /** Поле, в котором откроется правка. */
    focus: 'title' | 'amount';
    onedit: (id: number, focus: 'title' | 'amount') => void;
    oncycle: (id: number) => void;
    onsave: (patch: { title: string; amount: number }) => void;
    oncancel: () => void;
    ondelete: (id: number) => void;
    onmenu: (id: number, point: { x: number; y: number }) => void;
    /** Для строки открыто контекстное меню. */
    menuTarget?: boolean;
  };

  const uid = $props.id();
  let {
    row,
    editing,
    focus,
    onedit,
    oncycle,
    onsave,
    oncancel,
    ondelete,
    onmenu,
    menuTarget = false
  }: Props = $props();

  /** Клик по сумме ставит курсор в сумму, по названию — в название; Enter на кнопке (без клика) — в сумму. */
  function openEdit(event: MouseEvent): void {
    const inName = event.target instanceof Element && event.target.closest('.name') !== null;
    onedit(row.id, inName ? 'title' : 'amount');
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<li
  class="row {row.status}"
  class:menu-target={menuTarget}
  in:rowIn
  out:rowOut
  oncontextmenu={(event) => {
    if (editing) return;
    event.preventDefault();
    onmenu(row.id, { x: event.clientX, y: event.clientY });
  }}
  onkeydown={(event) => {
    if (!editing && (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey))) {
      event.preventDefault();
      const box = event.currentTarget.getBoundingClientRect();
      onmenu(row.id, { x: box.left + 24, y: box.bottom });
      return;
    }
    deleteOnKey(event, () => {
      ondelete(row.id);
    });
  }}
>
  <button
    type="button"
    class="status"
    aria-label="{STATUS_LABEL[row.status]}. Сменить статус"
    title={STATUS_LABEL[row.status]}
    onclick={() => {
      oncycle(row.id);
    }}
  >
    <!-- смена статуса: значок «щёлкает» (scale 0,6 → 1 со spring), фон строки перетекает -->
    {#key row.status}<span class="status-icon"><StatusDot status={row.status} /></span>{/key}
  </button>
  {#if editing}
    <InlineEdit {row} {focus} {onsave} {oncancel} />
  {:else}
    <button type="button" class="body" onclick={openEdit}>
      <span class="name" id="tx-name-{uid}">{row.title}</span>
      <span class="num sum">{formatMoney(row.amount)}</span>
    </button>
    <button
      type="button"
      class="remove reveal-action"
      aria-label="Удалить"
      aria-describedby="tx-name-{uid}"
      title="Удалить"
      onclick={() => {
        ondelete(row.id);
      }}>×</button
    >
  {/if}
</li>

<style>
  .status-icon {
    display: inline-flex;
    animation: status-pop var(--dur-base) var(--ease-spring);
  }
  @keyframes status-pop {
    from {
      transform: scale(0.6);
    }
  }
  .row {
    transition: background-color var(--dur-base) var(--ease-out);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    height: 34px;
    padding-inline: var(--sp-2);
    border-radius: var(--r-md);
    font-size: var(--fs-13);
  }
  .menu-target {
    box-shadow: inset 0 0 0 1px var(--primary);
  }
  .paid {
    background: var(--paid-bg);
  }
  .debt {
    background: var(--debt-bg);
  }
  .unplanned {
    background: var(--unpl-bg);
  }
  .planned {
    background: transparent;
  }
  .status,
  .body {
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .status {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    min-height: 24px;
    padding: var(--sp-1);
    border-radius: var(--r-sm);
  }
  .body {
    display: flex;
    flex-grow: 1;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
    height: 100%;
    padding: 0;
    text-align: left;
  }
  .name {
    flex-grow: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .remove {
    flex-shrink: 0;
    min-width: 24px;
    min-height: 24px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-16, 16px);
    line-height: 1;
    cursor: pointer;
  }
  .remove:hover {
    background: var(--line-2);
    color: var(--text);
  }
  .sum {
    font-weight: var(--fw-medium);
  }
</style>

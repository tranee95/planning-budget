<script lang="ts">
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
  };

  const uid = $props.id();
  let { row, editing, focus, onedit, oncycle, onsave, oncancel, ondelete }: Props = $props();

  /** Клик по сумме ставит курсор в сумму, по названию — в название; Enter на кнопке (без клика) — в сумму. */
  function openEdit(event: MouseEvent): void {
    const inName = event.target instanceof Element && event.target.closest('.name') !== null;
    onedit(row.id, inName ? 'title' : 'amount');
  }
</script>

<li class="row {row.status}">
  <button
    type="button"
    class="status"
    aria-label="{STATUS_LABEL[row.status]}. Сменить статус"
    title={STATUS_LABEL[row.status]}
    onclick={() => {
      oncycle(row.id);
    }}
  >
    <StatusDot status={row.status} />
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
      class="remove"
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
  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    height: 34px;
    padding-inline: var(--sp-2);
    border-radius: var(--r-md);
    font-size: var(--fs-13);
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

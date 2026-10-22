<script lang="ts">
  import { deleteOnKey } from '$lib/components/row-delete';
  import { untrack } from 'svelte';
  import type { TransactionDto } from '$lib/api/bindings';
  import { leftRow } from '$lib/components/amount-rows/row-focus';
  import { formatMoneyTrim } from '$lib/format';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { getSavingsVm } from '../savings.svelte';
  import RowField from './RowField.svelte';

  type Props = { row: TransactionDto };

  let { row }: Props = $props();

  const vm = getSavingsVm();
  const editing = $derived(vm.editingId === row.id);

  let box = $state<HTMLDivElement>();
  let amountField = $state<HTMLInputElement>();
  // Поля заполняются при входе в правку: обновление строки снаружи не стирает ввод.
  let title = $state(untrack(() => row.title));
  let amount = $state<number | null>(untrack(() => row.amount));
  const invalid = $derived(amount === null || amount <= 0);

  function begin(focus: 'title' | 'amount'): void {
    title = row.title;
    amount = row.amount;
    vm.startEdit(row.id);
    queueMicrotask(() => {
      if (focus === 'amount') amountField?.focus();
      else box?.querySelector('input')?.focus();
    });
  }

  function save(): void {
    if (invalid || amount === null) return;
    void vm.saveEdit({ title, amount });
  }

  /** Клик в другую область: изменённая строка сохраняется, нетронутая просто закрывается. */
  function onFocusOut(event: FocusEvent): void {
    if (!leftRow(event, box)) return;
    if (amount === row.amount && title === row.title) vm.cancelEdit();
    else if (!invalid) save();
  }
</script>

{#if editing}
  <div class="row" bind:this={box} onfocusout={onFocusOut}>
    <RowField
      label="Комментарий"
      placeholder={PLACEHOLDERS.savingNote}
      bind:value={title}
      onenter={save}
      onescape={() => {
        vm.cancelEdit();
      }}
    />
    <RowField
      label="Сумма, ₽"
      kind="money"
      placeholder={PLACEHOLDERS.rowAmount}
      bind:value={amount}
      bind:el={amountField}
      {invalid}
      onenter={save}
      onescape={() => {
        vm.cancelEdit();
      }}
    />
  </div>
{:else}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="row view"
    onkeydown={(event) => {
      deleteOnKey(event, () => {
        void vm.remove(row.id);
      });
    }}
  >
    <button
      type="button"
      class="cell title"
      onclick={() => {
        begin('title');
      }}>{row.title}</button
    >
    <button
      type="button"
      class="cell sum num"
      aria-label="Изменить сумму: {row.title}, {formatMoneyTrim(row.amount)}"
      onclick={() => {
        begin('amount');
      }}>{formatMoneyTrim(row.amount)}</button
    >
    <button
      type="button"
      class="remove reveal-action"
      aria-label="Удалить запись: {row.title}, {formatMoneyTrim(row.amount)}"
      title="Удалить"
      onclick={() => {
        void vm.remove(row.id);
      }}><span aria-hidden="true">×</span></button
    >
  </div>
{/if}

<style>
  .row {
    display: grid;
    grid-template-columns: var(--savings-cols);
    align-items: center;
    column-gap: var(--sp-3);
    padding-inline: var(--sp-3);
  }
  .view {
    min-height: var(--h-control);
    border-top: 1px solid var(--line-2);
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .view:hover,
  .view:focus-within {
    background: var(--surface-2);
  }
  .cell {
    min-width: 0;
    min-height: 32px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-14);
    cursor: text;
  }
  .title {
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sum {
    font-weight: var(--fw-medium);
    text-align: right;
    white-space: nowrap;
  }
  .remove {
    justify-self: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-17);
    line-height: 1;
    cursor: pointer;
  }
  .remove:hover {
    background: var(--line-2);
    color: var(--ink);
  }
</style>

<script lang="ts">
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { getSavingsVm } from '../savings.svelte';
  import RowField from './RowField.svelte';

  type Props = { categoryId: number };

  let { categoryId }: Props = $props();

  const vm = getSavingsVm();

  let title = $state('');
  let amount = $state<number | null>(null);
  let amountField = $state<HTMLInputElement>();
  let failed = $state(false);
  const errorId = $derived(`saving-entry-error-${String(categoryId)}`);

  async function submit(): Promise<void> {
    if (amount === null || amount <= 0) {
      failed = true;
      amountField?.focus();
      return;
    }
    vm.addingTo = categoryId;
    vm.draftTitle = title;
    vm.draftAmount = amount;
    if (await vm.commitAdd(true)) {
      title = '';
      amount = null;
      failed = false;
      amountField?.focus();
    }
  }
</script>

<div class="entry">
  <RowField
    label="Комментарий"
    placeholder={PLACEHOLDERS.savingNote}
    bind:value={title}
    onenter={submit}
  />
  <RowField
    label="Сумма, ₽"
    kind="money"
    placeholder={PLACEHOLDERS.rowAmount}
    bind:value={amount}
    bind:el={amountField}
    invalid={failed && (amount === null || amount <= 0)}
    describedby={errorId}
    onenter={submit}
  />
  <button
    type="button"
    class="add"
    aria-label="Добавить сумму"
    title="Добавить"
    onclick={() => {
      void submit();
    }}><span aria-hidden="true">+</span></button
  >
  <!-- место под сообщение занято всегда: карточка не прыгает по высоте, текст в одну строку -->
  <p class="error" id={errorId} role="alert">
    {#if failed && (amount === null || amount <= 0)}Введите сумму больше нуля{/if}
  </p>
</div>

<style>
  .entry {
    display: grid;
    grid-template-columns: var(--savings-cols);
    align-items: center;
    column-gap: var(--sp-3);
    row-gap: var(--sp-1);
    padding: var(--sp-3);
    border-top: 1px solid var(--line-2);
  }
  .add {
    justify-self: center;
    width: var(--h-control);
    height: var(--h-control);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--fs-17);
    line-height: 1;
    cursor: pointer;
  }
  .add:hover {
    background: var(--line-2);
  }
  .add:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
  }
  .error {
    grid-column: 1 / 3;
    min-height: 1.4em;
    margin: 0;
    white-space: nowrap;
    color: var(--unpl);
    font-size: var(--fs-12);
    text-align: right;
  }
</style>

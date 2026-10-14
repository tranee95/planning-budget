<script lang="ts">
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { untrack } from 'svelte';
  import type { TransactionDto } from '$lib/api/bindings';
  import { formatMoneyExact, parseMoney } from '$lib/format';
  import { leftRow } from './row-focus';

  type Props = {
    row: TransactionDto;
    /** Поле, в котором стоит курсор при открытии. */
    focus?: 'title' | 'amount';
    onsave: (patch: { title: string; amount: number }) => void;
    oncancel: () => void;
  };

  let { row, focus = 'title', onsave, oncancel }: Props = $props();

  // Поля заполняются один раз при входе в правку: обновление строки в списке не стирает ввод.
  let box = $state<HTMLDivElement>();
  let title = $state(untrack(() => row.title));
  let amountText = $state(untrack(() => formatMoneyExact(row.amount).replace(/\s?₽$/, '')));
  const amount = $derived(parseMoney(amountText));

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      if (amount !== null) onsave({ title, amount });
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      oncancel();
    }
  }

  /** Клик в другую область: изменённая строка сохраняется, нетронутая просто закрывается. */
  function onFocusOut(event: FocusEvent): void {
    if (!leftRow(event, box) || amount === null) return;
    if (title === row.title && amount === row.amount) oncancel();
    else onsave({ title, amount });
  }

  function focusOnMount(node: HTMLInputElement, wanted: boolean): void {
    if (!wanted) return;
    node.focus();
    node.select();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="edit" bind:this={box} onkeydown={onKeydown} onfocusout={onFocusOut}>
  <input
    class="title"
    aria-label="Наименование"
    placeholder={PLACEHOLDERS.rowTitle}
    autocomplete="off"
    bind:value={title}
    use:focusOnMount={focus === 'title'}
  />
  <input
    class="amount num"
    aria-label="Сумма, ₽"
    placeholder={PLACEHOLDERS.rowAmount}
    inputmode="decimal"
    autocomplete="off"
    aria-invalid={amount === null ? 'true' : undefined}
    bind:value={amountText}
    use:focusOnMount={focus === 'amount'}
  />
</div>

<style>
  .edit {
    display: flex;
    flex-grow: 1;
    gap: var(--sp-2);
    min-width: 0;
  }
  input {
    min-width: 0;
    height: 28px;
    padding-inline: var(--sp-2);
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }
  .title {
    flex-grow: 1;
  }
  .amount {
    width: 110px;
    text-align: right;
  }
  input[aria-invalid='true'] {
    border-color: var(--unpl);
  }
</style>

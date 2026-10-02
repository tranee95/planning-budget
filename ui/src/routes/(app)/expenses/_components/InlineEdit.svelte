<script lang="ts">
  import { untrack } from 'svelte';
  import type { TransactionDto } from '$lib/api/bindings';
  import { formatMoneyExact, parseMoney } from '$lib/format';

  type Props = {
    row: TransactionDto;
    onsave: (patch: { title: string; amount: number }) => void;
    oncancel: () => void;
  };

  let { row, onsave, oncancel }: Props = $props();

  // Поля заполняются один раз при входе в правку: обновление строки в списке не стирает ввод.
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

  function focusOnMount(node: HTMLInputElement): void {
    node.focus();
    node.select();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="edit" onkeydown={onKeydown}>
  <input
    class="title"
    aria-label="Наименование"
    autocomplete="off"
    bind:value={title}
    use:focusOnMount
  />
  <input
    class="amount num"
    aria-label="Сумма, ₽"
    inputmode="decimal"
    autocomplete="off"
    aria-invalid={amount === null ? 'true' : undefined}
    bind:value={amountText}
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

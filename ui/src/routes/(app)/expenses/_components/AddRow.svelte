<script lang="ts">
  import { parseMoney } from '$lib/format';

  type Props = {
    categoryName: string;
    onsubmit: (draft: { title: string; amount: number }) => void;
    oncancel: () => void;
  };

  let { categoryName, onsubmit, oncancel }: Props = $props();

  let title = $state('');
  let amountText = $state('');
  const amount = $derived(parseMoney(amountText));

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      if (amount !== null) onsubmit({ title, amount });
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      oncancel();
    }
  }

  function focusOnMount(node: HTMLInputElement): void {
    node.focus();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<li class="row" onkeydown={onKeydown}>
  <input
    class="title"
    aria-label="Наименование, {categoryName}"
    placeholder="Наименование"
    autocomplete="off"
    bind:value={title}
    use:focusOnMount
  />
  <input
    class="amount num"
    aria-label="Сумма, ₽"
    placeholder="0"
    inputmode="decimal"
    autocomplete="off"
    aria-invalid={amountText !== '' && amount === null ? 'true' : undefined}
    bind:value={amountText}
  />
</li>

<style>
  .row {
    display: flex;
    gap: var(--sp-2);
    height: 34px;
    padding-inline: var(--sp-2);
    list-style: none;
  }
  input {
    min-width: 0;
    height: 28px;
    padding-inline: var(--sp-2);
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface);
    color: var(--ink);
    font-size: var(--fs-13);
  }
  .title {
    flex-grow: 1;
  }
  .amount {
    width: 96px;
    text-align: right;
  }
  input[aria-invalid='true'] {
    border-color: var(--unpl);
  }
</style>

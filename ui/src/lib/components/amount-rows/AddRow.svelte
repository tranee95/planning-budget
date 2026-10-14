<script lang="ts">
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { parseMoney } from '$lib/format';
  import { leftRow } from './row-focus';

  type Props = {
    categoryName: string;
    /**
     * `again` — Enter при `keepOpen`: родитель оставляет строку открытой. Если он вернул
     * `false`, введённое не сохранилось и остаётся в полях.
     */
    onsubmit: (draft: { title: string; amount: number }, again: boolean) => unknown;
    oncancel: () => void;
    /** Enter сохраняет сумму и оставляет строку открытой с фокусом на сумме; Esc закрывает. */
    keepOpen?: boolean;
  };

  let { categoryName, onsubmit, oncancel, keepOpen = false }: Props = $props();

  let row = $state<HTMLLIElement>();
  let amountInput = $state<HTMLInputElement>();
  let title = $state('');
  let amountText = $state('');
  let pending = false;
  const amount = $derived(parseMoney(amountText));

  async function submitAgain(draft: { title: string; amount: number }): Promise<void> {
    if (pending) return;
    pending = true;
    try {
      if ((await onsubmit(draft, true)) === false) return;
      title = '';
      amountText = '';
      amountInput?.focus();
    } finally {
      pending = false;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      if (amount === null) return;
      if (keepOpen) void submitAgain({ title, amount });
      else void onsubmit({ title, amount }, false);
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      oncancel();
    }
  }

  /** Клик в другую область: сумма сохраняется сама; пустая строка закрывается без следа. */
  function onFocusOut(event: FocusEvent): void {
    if (!leftRow(event, row)) return;
    if (title.trim() === '' && amountText.trim() === '') oncancel();
    else if (amount !== null && amount > 0 && !pending) void onsubmit({ title, amount }, false);
  }

  /** Первой вводят сумму; название необязательно и идёт следом (Shift+Tab). */
  function focusOnMount(node: HTMLInputElement): void {
    node.focus();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<li class="row" bind:this={row} onkeydown={onKeydown} onfocusout={onFocusOut}>
  <input
    class="title"
    aria-label="Наименование, {categoryName}"
    placeholder={PLACEHOLDERS.rowTitle}
    autocomplete="off"
    bind:value={title}
  />
  <input
    class="amount num"
    aria-label="Сумма, ₽"
    placeholder={PLACEHOLDERS.rowAmount}
    inputmode="decimal"
    autocomplete="off"
    aria-invalid={amountText !== '' && amount === null ? 'true' : undefined}
    bind:value={amountText}
    bind:this={amountInput}
    use:focusOnMount
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

<script lang="ts">
  import { untrack } from 'svelte';
  import { getSavingsVm, type SavingCard } from '../savings.svelte';

  type Props = { card: SavingCard };

  let { card }: Props = $props();

  const vm = getSavingsVm();

  const renaming = $derived(vm.renamingId === card.categoryId);
  // Поле заполняется при входе в правку: обновление карточки снаружи не стирает ввод.
  let draft = $state(untrack(() => card.name));
  /** Esc и Enter уже решили судьбу правки: уход фокуса после них ничего не пишет. */
  let settled = false;

  function begin(): void {
    draft = card.name;
    settled = false;
    vm.startRename(card.categoryId);
  }

  function finish(): void {
    if (settled) return;
    settled = true;
    void vm.rename(card.categoryId, draft);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      finish();
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      settled = true;
      vm.cancelRename();
    }
  }

  function focusOnMount(node: HTMLInputElement): void {
    node.focus();
    node.select();
  }
</script>

<h3>
  {#if renaming}
    <input
      class="field ring-field"
      aria-label="Название накопления"
      autocomplete="off"
      bind:value={draft}
      onkeydown={onKeydown}
      onblur={() => {
        // Alt+Tab не считается уходом: ввод остаётся, пока пользователь не вернётся.
        if (document.hasFocus()) finish();
      }}
      use:focusOnMount
    />
  {:else}
    <button
      type="button"
      class="name"
      aria-label="Переименовать накопление «{card.name}»"
      onclick={begin}
    >
      {card.name}
    </button>
  {/if}
</h3>

<style>
  h3 {
    flex-grow: 1;
    min-width: 0;
    margin: 0;
    font-size: var(--fs-14);
    font-weight: var(--fw-semibold);
  }
  .name {
    max-width: 100%;
    min-height: 24px;
    padding: 0 var(--sp-1);
    margin-inline: calc(var(--sp-1) * -1);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: text;
  }
  .name:hover {
    background: var(--line-2);
  }
  .field {
    width: 100%;
    min-height: 28px;
    padding: 0 var(--sp-2);
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }
</style>

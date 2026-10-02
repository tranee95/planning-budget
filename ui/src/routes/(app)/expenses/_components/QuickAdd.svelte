<script lang="ts">
  import { Button, Kbd, Modal, MoneyInput, StatusSegmented } from '$lib/components';
  import type { TxStatusDto } from '$lib/api/bindings';
  import { formatMoney, monthRange } from '$lib/format';
  import { getExpensesVm } from '../expenses.svelte';
  import { QuickAddVm } from '../quick-add.svelte';

  const expenses = getExpensesVm();
  // Состояние формы живёт, пока открыта модалка: новый вызов — чистая форма.
  const vm = new QuickAddVm(expenses);
  $effect(() => () => {
    vm.dispose();
  });
  const range = $derived(monthRange(expenses.month));

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Enter' || event.isComposing) return;
    const target = event.target as HTMLElement;
    // Enter на кнопке и в списке подсказок — их собственное действие.
    if (target.tagName === 'BUTTON') return;
    event.preventDefault();
    void vm.save(event.shiftKey);
  }

  function onChipKeydown(event: KeyboardEvent, index: number): void {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = vm.chips[(index + step + vm.chips.length) % vm.chips.length];
    if (!next) return;
    vm.categoryId = next.id;
    const group = (event.currentTarget as HTMLElement).parentElement;
    group?.querySelectorAll<HTMLElement>('[role=radio]')[vm.chips.indexOf(next)]?.focus();
  }
</script>

<Modal
  title="Новая трата"
  onclose={() => {
    expenses.closeEditor();
  }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="form" onkeydown={onKeydown}>
    <MoneyInput id="qa-amount" label="Сумма" size="lg" autofocus bind:value={vm.amount} />

    <div class="group">
      <span class="lbl" id="qa-category">Категория</span>
      <div class="chips" role="radiogroup" aria-labelledby="qa-category">
        {#each vm.chips as chip, index (chip.id)}
          <button
            type="button"
            role="radio"
            class="chip"
            aria-checked={vm.categoryId === chip.id}
            tabindex={vm.categoryId === chip.id || (vm.categoryId === null && index === 0) ? 0 : -1}
            onclick={() => {
              vm.categoryId = chip.id;
            }}
            onkeydown={(event) => {
              onChipKeydown(event, index);
            }}>{chip.name}</button
          >
        {/each}
      </div>
      {#if vm.limitWarning}
        <p class="warn" role="status">
          После этой траты «{vm.limitWarning.category}» превысит лимит на
          <b class="num">{formatMoney(vm.limitWarning.over)}</b>
        </p>
      {/if}
    </div>

    <div class="row">
      <div class="group">
        <label class="lbl" for="qa-title">Наименование</label>
        <input id="qa-title" class="field" autocomplete="off" bind:value={vm.title} />
        {#if vm.suggestions.length > 0}
          <ul class="suggest" aria-label="Подсказки из истории">
            {#each vm.suggestions as suggestion (suggestion.title)}
              <li>
                <button
                  type="button"
                  onclick={() => {
                    vm.pickSuggestion(suggestion);
                  }}>{suggestion.title}</button
                >
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      <div class="group">
        <label class="lbl" for="qa-date">Дата</label>
        <input
          id="qa-date"
          class="field"
          type="date"
          min={range.first}
          max={range.last}
          aria-invalid={vm.dateError !== null}
          bind:value={vm.date}
        />
        {#if vm.dateError}<span class="err" role="alert">{vm.dateError}</span>{/if}
      </div>
    </div>

    <div class="group">
      <span class="lbl">Статус</span>
      <StatusSegmented
        value={vm.status}
        onchange={(status: TxStatusDto) => {
          vm.status = status;
        }}
      />
    </div>
  </div>

  {#snippet footer()}
    <Button
      variant="secondary"
      size="lg"
      disabled={!vm.canSave}
      onclick={() => {
        void vm.save(true);
      }}>Ещё одну <Kbd>⇧↵</Kbd></Button
    >
    <Button
      size="lg"
      loading={vm.saving}
      disabled={!vm.canSave}
      onclick={() => {
        void vm.save(false);
      }}>Сохранить <Kbd>↵</Kbd></Button
    >
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    min-width: 0;
  }
  .lbl {
    color: var(--muted);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: var(--sp-3);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }
  .chip {
    height: 32px;
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-full);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-13);
    cursor: pointer;
  }
  .chip[aria-checked='true'] {
    border-color: var(--ink);
    background: var(--ink);
    color: var(--bg);
  }
  .chip:focus-visible,
  .suggest button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .warn {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin: 0;
    padding: var(--sp-3);
    border-radius: var(--r-md);
    background: var(--unpl-bg);
    color: var(--unpl);
    font-size: var(--fs-13);
  }
  .field {
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-14);
  }
  .err {
    color: var(--unpl);
    font-size: var(--fs-12);
  }
  .field[aria-invalid='true'] {
    border-color: var(--unpl);
  }
  .field:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .suggest {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .suggest button {
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: var(--line-2);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-12);
    cursor: pointer;
  }
</style>

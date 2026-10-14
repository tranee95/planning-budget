<script lang="ts">
  import { Button, MoneyInput } from '$lib/components';
  import AddRow from '$lib/components/amount-rows/AddRow.svelte';
  import InlineEdit from '$lib/components/amount-rows/InlineEdit.svelte';
  import { formatMoney, formatMonthPrepositional } from '$lib/format';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { getSavingsVm, type SavingCard } from '../savings.svelte';
  import SavingName from './SavingName.svelte';

  type Props = { card: SavingCard };

  let { card }: Props = $props();

  const vm = getSavingsVm();

  /** Поле плана: значение берётся из плана месяца и обновляется, когда он изменился снаружи. */
  let planDraft = $derived<number | null>(card.plan || null);
  const planChanged = $derived((planDraft ?? 0) !== card.plan);
</script>

<article class="card" aria-label={card.name}>
  <header>
    <SavingName {card} />
    <Button
      size="sm"
      variant="ghost"
      aria-label="Закрыть цель {card.name}"
      onclick={() => {
        vm.askClose(card.categoryId);
      }}>Закрыть цель</Button
    >
    <span class="total">
      Отложено в {formatMonthPrepositional(vm.month)}: <b class="num">{formatMoney(card.total)}</b>
    </span>
  </header>

  <ul class="amounts">
    {#each card.amounts as row (row.id)}
      <li class="amount">
        {#if vm.editingId === row.id}
          <InlineEdit
            {row}
            onsave={(patch: { title: string; amount: number }) => {
              void vm.saveEdit(patch);
            }}
            oncancel={() => {
              vm.cancelEdit();
            }}
          />
        {:else}
          <button
            type="button"
            class="body"
            onclick={() => {
              vm.startEdit(row.id);
            }}
          >
            <span class="name">{row.title}</span>
            <span class="num sum">{formatMoney(row.amount)}</span>
          </button>
          <button
            type="button"
            class="remove"
            aria-label="Удалить"
            aria-describedby="saving-amount-{row.id}"
            title="Удалить"
            onclick={() => {
              void vm.remove(row.id);
            }}>×</button
          >
          <span class="sr-only" id="saving-amount-{row.id}"
            >{row.title}, {formatMoney(row.amount)}</span
          >
        {/if}
      </li>
    {:else}
      <li class="empty">В {formatMonthPrepositional(vm.month)} ещё ничего не откладывали.</li>
    {/each}
    {#if vm.addingTo === card.categoryId}
      <AddRow
        categoryName={card.name}
        keepOpen
        onsubmit={(draft: { title: string; amount: number }, again: boolean) => {
          vm.draftTitle = draft.title;
          vm.draftAmount = draft.amount;
          return vm.commitAdd(again);
        }}
        oncancel={() => {
          vm.cancelAdd();
        }}
      />
    {/if}
  </ul>

  <footer>
    <Button
      size="sm"
      variant="success"
      aria-label="Добавить сумму в {card.name}"
      onmousedown={(event: MouseEvent) => {
        // Строка ввода уже открыта: фокус остаётся в ней, кнопка не сдвигается из-под курсора.
        if (vm.addingTo === card.categoryId) event.preventDefault();
      }}
      onclick={() => {
        vm.startAdd(card.categoryId);
      }}><span aria-hidden="true">+</span> Добавить сумму</Button
    >
    <div class="plan">
      <MoneyInput
        id="saving-plan-{card.categoryId}"
        label="План на месяц"
        placeholder={PLACEHOLDERS.savingAmount}
        bind:value={planDraft}
      />
      <Button
        size="sm"
        disabled={!planChanged}
        onclick={() => {
          void vm.setPlan(card.categoryId, planDraft ?? 0);
        }}>Сохранить план</Button
      >
    </div>
  </footer>
</article>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-2);
  }
  .total {
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .total b {
    color: var(--ink);
  }
  .amounts {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .amount {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    height: 34px;
    padding-inline: var(--sp-2);
    border-radius: var(--r-md);
    background: var(--paid-bg);
    font-size: var(--fs-13);
  }
  .empty {
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .body {
    display: flex;
    flex-grow: 1;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .name {
    flex-grow: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sum {
    font-weight: var(--fw-medium);
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
    color: var(--ink);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--sp-3);
  }
  .plan {
    display: flex;
    align-items: flex-end;
    gap: var(--sp-2);
  }
</style>

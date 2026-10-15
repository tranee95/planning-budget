<script lang="ts">
  import { leftRow } from '$lib/components/amount-rows/row-focus';
  import { formatMoneyTrim, formatMonthPrepositional } from '$lib/format';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { getSavingsVm, type SavingCard } from '../savings.svelte';
  import GoalMenu from './GoalMenu.svelte';
  import RowField from './RowField.svelte';
  import SavingEntry from './SavingEntry.svelte';
  import SavingName from './SavingName.svelte';
  import SavingRecord from './SavingRecord.svelte';

  type Props = { card: SavingCard };

  let { card }: Props = $props();

  const vm = getSavingsVm();

  const open = $derived(vm.isOpen(card.categoryId));
  const bodyId = $derived(`saving-body-${String(card.categoryId)}`);
  const editingPlan = $derived(vm.planEditingId === card.categoryId);
  /** Доля плана в целых процентах: только подпись и ширина полосы, сумм она не меняет. */
  const percent = $derived(card.plan > 0 ? Math.floor((card.total * 100) / card.plan) : null);
  const done = $derived(percent !== null && percent >= 100);

  // Поле плана заполняется при входе в правку: обновление карточки снаружи не стирает ввод.
  let planDraft = $state<number | null>(null);
  let planField = $state<HTMLInputElement>();
  /** Enter или Esc уже решили судьбу правки: уход фокуса после них ничего не пишет. */
  let settled = false;

  function beginPlan(): void {
    planDraft = card.plan || null;
    settled = false;
    vm.startPlanEdit(card.categoryId);
  }

  function finishPlan(save: boolean): void {
    if (settled) return;
    settled = true;
    if (save) void vm.savePlan(card.categoryId, planDraft);
    else vm.cancelPlanEdit();
  }

  $effect(() => {
    if (editingPlan) planField?.focus();
  });
</script>

<article class="card" aria-label={card.name}>
  <header>
    <button
      type="button"
      class="chevron"
      class:open
      aria-expanded={open}
      aria-controls={bodyId}
      aria-label="{open ? 'Свернуть' : 'Развернуть'} накопление «{card.name}»"
      onclick={() => {
        vm.toggle(card.categoryId);
      }}><span aria-hidden="true">›</span></button
    >
    <SavingName {card} />
    <div class="summary">
      <b class="num">{formatMoneyTrim(card.total)}</b>
      <span class="sep">/</span>
      {#if editingPlan}
        <div
          class="plan-edit"
          onfocusout={(event) => {
            if (leftRow(event, event.currentTarget)) finishPlan(true);
          }}
        >
          <RowField
            label="План на месяц, ₽"
            kind="money"
            placeholder={PLACEHOLDERS.savingAmount}
            bind:value={planDraft}
            bind:el={planField}
            onenter={() => {
              finishPlan(true);
            }}
            onescape={() => {
              finishPlan(false);
            }}
          />
        </div>
      {:else if vm.locked}
        <span class="num plan-static">{formatMoneyTrim(card.plan)}</span>
      {:else}
        <button
          type="button"
          class="plan num"
          aria-label="Изменить план на месяц «{card.name}»: {card.plan > 0
            ? formatMoneyTrim(card.plan)
            : 'не задан'}"
          onclick={beginPlan}>{card.plan > 0 ? formatMoneyTrim(card.plan) : 'задать план'}</button
        >
      {/if}
      {#if percent !== null}<span class="pct num">· {percent}%</span>{/if}
    </div>
    <GoalMenu
      name={card.name}
      items={[
        { label: 'Изменить план', disabled: vm.locked, run: beginPlan },
        {
          label: 'Закрыть цель',
          run: () => {
            vm.askClose(card.categoryId);
          }
        },
        {
          label: 'Удалить цель',
          danger: true,
          run: () => {
            vm.askDelete(card.categoryId);
          }
        }
      ]}
    />
  </header>

  {#if percent !== null}
    <div
      class="bar"
      class:done
      role="progressbar"
      aria-label="Выполнение плана «{card.name}»"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.min(percent, 100)}
    >
      <span class="fill" style:transform="scaleX({Math.min(percent, 100) / 100})"></span>
    </div>
    {#if done}<p class="done-note">План выполнен</p>{/if}
  {/if}

  {#if open}
    <div class="body" id={bodyId}>
      {#if vm.locked}
        <p class="locked">
          План зафиксирован, изменить его можно после «Разблокировать» на Обзоре.
        </p>
      {/if}
      {#each card.amounts as row (row.id)}
        <SavingRecord {row} />
      {:else}
        <p class="empty">Пока ничего не отложено в {formatMonthPrepositional(vm.month)}</p>
      {/each}
      <SavingEntry categoryId={card.categoryId} />
    </div>
  {/if}
</article>

<style>
  .card {
    --savings-cols: minmax(0, 1fr) 160px 40px;
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-4) 0 0;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding-inline: var(--sp-4);
  }
  .chevron {
    flex: none;
    width: var(--h-control-sm);
    height: var(--h-control-sm);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-22);
    line-height: 1;
    cursor: pointer;
    transition: transform var(--dur-fast) var(--ease-out);
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .chevron:hover {
    background: var(--line-2);
    color: var(--ink);
  }
  .summary {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--sp-1);
    font-size: var(--fs-14);
    white-space: nowrap;
  }
  .sep,
  .pct {
    color: var(--muted);
  }
  .plan {
    min-height: 32px;
    padding: 0 var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink);
    font: inherit;
    cursor: text;
    text-decoration: underline dotted var(--muted);
    text-underline-offset: 4px;
  }
  .plan:hover {
    background: var(--line-2);
  }
  .plan-edit {
    width: 160px;
  }
  .bar {
    height: 6px;
    margin-inline: var(--sp-4);
    overflow: hidden;
    border-radius: var(--r-full);
    background: var(--line-2);
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--paid-bar);
    transform-origin: left;
    transition: transform var(--dur-base) var(--ease-out);
  }
  .done-note {
    margin: calc(var(--sp-2) * -1) 0 0;
    padding-inline: var(--sp-4);
    color: var(--paid);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .body {
    display: flex;
    flex-direction: column;
  }
  .empty,
  .locked {
    margin: 0;
    padding: var(--sp-2) var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .empty {
    padding-block: var(--sp-4);
    border-top: 1px solid var(--line-2);
  }
</style>

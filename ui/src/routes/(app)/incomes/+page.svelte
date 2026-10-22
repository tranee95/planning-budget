<script lang="ts">
  import { deleteOnKey } from '$lib/components/row-delete';
  import { rowIn, rowOut } from '$lib/motion';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { Button, EmptyState, FilterBar, FormulaHint, Segmented, Skeleton } from '$lib/components';
  import type { IncomeStatusDto } from '$lib/api/bindings';
  import { formatDay, formatMoney, formatMonth } from '$lib/format';
  import { FORMULAS } from '$lib/formulas';
  import { month } from '$lib/stores/month.svelte';
  import IncomeAdd from './_components/IncomeAdd.svelte';
  import { IncomesVm, setIncomesVm } from './incomes.svelte';

  const vm = new IncomesVm();
  setIncomesVm(vm);

  const FILTERS = [
    { value: 'all', label: 'Все' },
    { value: 'received', label: 'Получено' },
    { value: 'expected', label: 'Ожидается' }
  ] as const;

  const STATUS_LABEL: Record<IncomeStatusDto, string> = {
    received: 'Получено',
    expected: 'Ожидается'
  };

  // Ссылки палитры: `?q=` открывает список с запросом.
  $effect(() => {
    const query = page.url.searchParams.get('q');
    if (query === null) return;
    untrack(() => {
      vm.filter.set(query);
      void goto(resolve('/incomes'), { replaceState: true, noScroll: true, keepFocus: true });
    });
  });

  $effect(() => {
    // Запрос — зависимость эффекта: его смена перезагружает список.
    const [current] = [month.current, vm.filter.applied];
    void vm.load(current);
  });

  onMount(() => vm.connect());
</script>

<svelte:head>
  <title>Planning Budget — доходы</title>
</svelte:head>

<div class="screen">
  <div class="bar">
    <Segmented
      label="Показать"
      options={FILTERS}
      value={vm.statusFilter ?? 'all'}
      onchange={(value: 'all' | IncomeStatusDto) => {
        vm.statusFilter = value === 'all' ? null : value;
      }}
    />
    <span class="spacer"></span>
    <span class="num totals">
      Получено <b>{formatMoney(vm.totals.received)}</b> · ожидается
      <b>{formatMoney(vm.totals.expected)}</b> · всего <b>{formatMoney(vm.totals.total)}</b>
    </span>
    <FormulaHint label={FORMULAS.incomesTotals.label} lines={FORMULAS.incomesTotals.lines} />
    <Button
      variant="primary"
      onclick={() => {
        vm.openEditor();
      }}><span aria-hidden="true">+</span> Новый доход</Button
    >
  </div>

  <FilterBar
    filter={vm.filter}
    subject="доходов"
    screen="incomes"
    placeholder={PLACEHOLDERS.searchIncomes}
  />

  {#if vm.error}
    <EmptyState title="Не удалось загрузить доходы" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load(month.current);
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.loading && vm.incomes.length === 0}
    <Skeleton height="160px" />
  {:else if vm.isEmpty && !vm.filtered}
    <EmptyState title="В этом месяце доходов нет" hint="Добавьте зарплату или другое поступление.">
      {#snippet actions()}
        <Button
          variant="primary"
          onclick={() => {
            vm.openEditor();
          }}><span aria-hidden="true">+</span> Новый доход</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.visible.length === 0}
    <EmptyState
      title={vm.filtered ? 'Ничего не найдено' : 'Нет доходов с таким статусом'}
      hint={vm.filtered ? 'Измените запрос или уберите лишние чипы.' : undefined}
    />
  {:else}
    <ul class="list" aria-label="Доходы месяца">
      {#each vm.visible as income (income.id)}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <li
          class="item"
          in:rowIn
          out:rowOut
          onkeydown={(event) => {
            deleteOnKey(event, () => {
              void vm.remove(income.id);
            });
          }}
        >
          <button
            type="button"
            class="body"
            aria-label="Изменить доход «{income.sourceName}»"
            onclick={() => {
              vm.openEdit(income.id);
            }}
          >
            <span class="source"
              >{income.sourceName}{#if vm.filtered}<span class="muted when">
                  · {formatMonth(income.month)}</span
                >{:else if income.date !== null}<span class="muted when">
                  · {formatDay(income.date)}</span
                >{/if}</span
            >
            <span class="num amount">{formatMoney(income.amount)}</span>
          </button>
          <button
            type="button"
            class="status {income.status}"
            aria-label="{STATUS_LABEL[income.status]}: сменить статус дохода «{income.sourceName}»"
            onclick={() => {
              void vm.setStatus(income.id, income.status === 'received' ? 'expected' : 'received');
            }}>{STATUS_LABEL[income.status]}</button
          >
          <button
            type="button"
            class="remove reveal-action"
            aria-label="Удалить доход «{income.sourceName}»"
            title="Удалить"
            onclick={() => {
              void vm.remove(income.id);
            }}>×</button
          >
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if vm.editorOpen}
  <IncomeAdd />
{/if}

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-3);
  }
  .spacer {
    flex-grow: 1;
  }
  .totals {
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .totals b {
    color: var(--ink);
    font-weight: var(--fw-semibold);
  }
  .list {
    max-width: 880px; /* на широком окне строка не растягивается на весь экран */
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  .item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 120px 28px;
    align-items: center;
    gap: var(--sp-4);
    min-height: 48px;
    padding-inline: var(--sp-4);
  }
  .item + .item {
    border-top: 1px solid var(--line-2);
  }
  .muted {
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .body {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-4);
    min-width: 0;
    height: 100%;
    min-height: 48px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .body:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
    border-radius: var(--r-sm);
  }
  .remove {
    width: 24px;
    height: 24px;
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
  .source {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .amount {
    font-weight: var(--fw-semibold);
    text-align: end;
  }
  .status {
    height: 24px;
    padding-inline: var(--sp-3);
    border: 1px solid transparent;
    border-radius: var(--r-full);
    font: inherit;
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
    cursor: pointer;
  }
  .status:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
  }
  .received {
    background: var(--paid-bg);
    color: var(--paid);
  }
  .expected {
    border: 1px dashed var(--plan);
    background: transparent;
    color: var(--plan);
  }
</style>

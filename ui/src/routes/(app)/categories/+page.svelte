<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Button, EmptyState, FormulaHint, Modal, Skeleton, Switch } from '$lib/components';
  import { formatMoney, formatPercent } from '$lib/format';
  import { FORMULAS } from '$lib/formulas';
  import { month } from '$lib/stores/month.svelte';
  import CategoryEditor from './_components/CategoryEditor.svelte';
  import CategoryList from './_components/CategoryList.svelte';
  import { CategoriesVm, setCategoriesVm } from './categories.svelte';

  const vm = new CategoriesVm();
  setCategoriesVm(vm);

  $effect(() => {
    const current = month.current;
    untrack(() => {
      void vm.load(current);
    });
  });

  onMount(() => vm.connect());

  // Ни одной категории (даже архивной): одно действие — в пустом состоянии, не дублируем в шапке.
  const noCategories = $derived(!vm.loading && vm.items.length === 0 && vm.settings !== null);
</script>

<svelte:head>
  <title>Planning Budget — категории</title>
</svelte:head>

<div class="screen">
  <div class="bar">
    <p class="subtitle">{vm.subtitle}</p>
    <span class="spacer"></span>
    <Switch
      checked={vm.showArchived}
      onchange={(checked: boolean) => {
        vm.showArchived = checked;
      }}>Показать архив</Switch
    >
    {#if !noCategories}
      <Button
        variant="success"
        onclick={() => {
          vm.startCreate();
        }}><span aria-hidden="true">+</span> Добавить категорию</Button
      >
    {/if}
  </div>

  {#if vm.error}
    <EmptyState title="Не удалось загрузить категории" hint={vm.error}>
      {#snippet actions()}
        <Button
          variant="secondary"
          onclick={() => {
            void vm.load(month.current);
          }}>Повторить</Button
        >
      {/snippet}
    </EmptyState>
  {:else if vm.loading && vm.items.length === 0}
    <Skeleton height="320px" />
  {:else}
    <div class="body">
      {#if noCategories}
        <section class="list" aria-label="Категории">
          <EmptyState
            title="Категорий пока нет"
            hint="Категория объединяет траты и задаёт лимит на месяц. Добавьте первую."
          >
            {#snippet actions()}
              <Button
                variant="success"
                onclick={() => {
                  vm.startCreate();
                }}><span aria-hidden="true">+</span> Добавить категорию</Button
              >
            {/snippet}
          </EmptyState>
        </section>
      {:else}
        <section class="list" aria-label="Категории">
          <CategoryList />
          <div class="spacer"></div>
          {#if vm.balance !== null}
            <div class="balance">
              <span class="muted">
                Лимиты <b class="num">{formatMoney(vm.balance.limitsSum)}</b> · сбережения
                {#if vm.settings}{formatPercent(vm.settings.savingsNormBp)}{/if}
                <b class="num">{formatMoney(vm.balance.savingsTarget)}</b> · буфер
                <b class="num" class:neg={vm.balance.buffer < 0}>{formatMoney(vm.balance.buffer)}</b
                >
              </span>
              <FormulaHint
                label={FORMULAS.budgetBalance.label}
                lines={FORMULAS.budgetBalance.lines}
              />
            </div>
          {/if}
          <p class="keys">Порядок: потяните строку за ручку слева</p>
        </section>
      {/if}
      {#if vm.selected !== null}
        <CategoryEditor />
      {/if}
    </div>
  {/if}
</div>

{#if vm.archiveAsk}
  <Modal
    title="Отправить в архив?"
    width={460}
    onclose={() => {
      vm.cancelArchive();
    }}
  >
    <p class="ask">
      Категория «{vm.archiveAsk.name}» исчезнет из списков и быстрого добавления. Траты и история
      останутся, вернуть её можно через «Показать архив».
    </p>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          vm.cancelArchive();
        }}>Отмена</Button
      >
      <Button
        variant="primary"
        onclick={() => {
          void vm.confirmArchive();
        }}>В архив</Button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .ask {
    margin: 0;
    font-size: var(--fs-13);
    line-height: var(--lh-body);
  }
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
  .subtitle {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .spacer {
    flex-grow: 1;
  }
  .body {
    display: flex;
    gap: var(--sp-4);
    align-items: flex-start;
  }
  .list {
    container-type: inline-size;
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-2);
    min-width: 0;
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .balance {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-4) var(--sp-1);
    border-top: 1px solid var(--line);
    font-size: var(--fs-13);
  }
  .muted {
    color: var(--muted);
  }
  .muted b {
    color: var(--ink);
  }
  .neg {
    color: var(--unpl) !important;
  }
  .keys {
    margin: 0;
    padding: 0 var(--sp-4);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  @media (max-width: 1100px) {
    .body {
      flex-direction: column;
      align-items: stretch;
    }
    .body :global(.editor) {
      width: auto;
    }
  }
</style>

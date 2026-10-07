<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Button, EmptyState, Kbd, Skeleton, Switch } from '$lib/components';
  import { formatMoney, formatPercent } from '$lib/format';
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
    <Button
      onclick={() => {
        vm.startCreate();
      }}>Категория</Button
    >
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
      <section class="list" aria-label="Категории">
        <CategoryList />
        <div class="spacer"></div>
        {#if vm.balance !== null}
          <div class="balance">
            <span class="muted">
              Лимиты <b class="num">{formatMoney(vm.balance.limitsSum)}</b> · сбережения
              {#if vm.settings}{formatPercent(vm.settings.savingsNormBp)}{/if}
              <b class="num">{formatMoney(vm.balance.savingsTarget)}</b> · буфер
              <b class="num" class:neg={vm.balance.buffer < 0}>{formatMoney(vm.balance.buffer)}</b>
            </span>
            {#if vm.rateMismatch && vm.settings}
              <span class="warn">
                Сумма процентов категорий {formatPercent(vm.monthPlanBp ?? 0)} не равна норме {formatPercent(
                  vm.settings.savingsNormBp
                )}: план сбережений считается по категориям, баланс — по норме.
              </span>
            {/if}
          </div>
        {/if}
        <p class="keys">
          Порядок: перетащите строку или <Kbd>Alt</Kbd> + <Kbd>↑</Kbd>/<Kbd>↓</Kbd>
        </p>
      </section>
      {#if vm.selected !== null}
        <CategoryEditor />
      {/if}
    </div>
  {/if}
</div>

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
    gap: 6px;
    min-width: 0;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .balance {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px 4px;
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
  .warn {
    color: var(--debt);
  }
  .keys {
    margin: 0;
    padding: 0 14px;
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

<script lang="ts">
  import { Button } from '$lib/components';
  import { formatMoney, formatMonth } from '$lib/format';
  import { getSavingsVm } from '../savings.svelte';
  import NewGoalForm from './NewGoalForm.svelte';
  import SavingCard from './SavingCard.svelte';

  const vm = getSavingsVm();
</script>

<section class="list" aria-labelledby="savings-title">
  <header class="head">
    <h2 id="savings-title">Накопления</h2>
    <p class="month">
      Всего отложено в {formatMonth(vm.month).toLowerCase()}:
      <b class="num">{formatMoney(vm.monthTotal)}</b>
    </p>
    <Button
      variant="success"
      aria-expanded={vm.newOpen}
      onclick={() => {
        vm.newOpen = !vm.newOpen;
      }}><span aria-hidden="true">+</span> Новая цель</Button
    >
  </header>
  {#if vm.newOpen}<NewGoalForm />{/if}
  <ul class="cards">
    {#each vm.cards as card (card.categoryId)}
      <li><SavingCard {card} /></li>
    {:else}
      <li class="empty">Накоплений пока нет. Создайте первую цель кнопкой «Новая цель».</li>
    {/each}
  </ul>
  {#if vm.closed.length > 0}
    <div class="closed">
      <button
        type="button"
        class="closed-toggle"
        aria-expanded={vm.showClosed}
        aria-controls="closed-goals"
        onclick={() => {
          vm.showClosed = !vm.showClosed;
        }}
      >
        <span class="chev" class:open={vm.showClosed} aria-hidden="true">›</span>
        Закрытые цели ({vm.closed.length})
      </button>
      {#if vm.showClosed}
        <ul class="goals" id="closed-goals">
          {#each vm.closed as goal (goal.categoryId)}
            <li class="goal">
              <span class="goal-name">{goal.name}</span>
              <span class="goal-total">накоплено <b class="num">{formatMoney(goal.total)}</b></span>
              <Button
                size="sm"
                variant="secondary"
                aria-label="Вернуть цель {goal.name}"
                onclick={() => {
                  void vm.reopen(goal.categoryId);
                }}>Вернуть</Button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</section>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-width: 0;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-3);
  }
  h2 {
    margin: 0;
    margin-right: auto;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .month {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .month b {
    color: var(--ink);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 480px), 1fr));
    align-items: start;
    gap: var(--sp-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .cards > li {
    min-width: 0;
  }
  .closed {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .closed-toggle {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    gap: var(--sp-2);
    min-height: var(--h-control-sm);
    padding: 0 var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink-2);
    font: inherit;
    font-size: var(--fs-13);
    cursor: pointer;
  }
  .closed-toggle:hover {
    background: var(--line-2);
  }
  .chev {
    display: inline-block;
    transition: transform var(--dur-fast) var(--ease-out);
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .goals {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .goal {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    font-size: var(--fs-13);
  }
  .goal-name {
    flex-grow: 1;
    overflow: hidden;
    font-weight: var(--fw-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .goal-total {
    color: var(--muted);
  }
  .empty {
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>

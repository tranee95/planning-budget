<script lang="ts">
  import { Button, Switch, TextField } from '$lib/components';
  import { formatMoney } from '$lib/format';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { getSavingsVm } from '../savings.svelte';
  import SavingCard from './SavingCard.svelte';

  const vm = getSavingsVm();
</script>

<section class="list" aria-labelledby="savings-title">
  <h2 id="savings-title">Накопления</h2>
  <ul class="cards">
    {#each vm.cards as card (card.categoryId)}
      <li><SavingCard {card} /></li>
    {:else}
      <li class="empty">Накоплений пока нет. Добавьте первое ниже.</li>
    {/each}
  </ul>
  {#if vm.closed.length > 0}
    <div class="closed">
      <Switch
        checked={vm.showClosed}
        onchange={(checked: boolean) => {
          vm.showClosed = checked;
        }}>Показать закрытые цели ({vm.closed.length})</Switch
      >
      {#if vm.showClosed}
        <ul class="goals">
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
  <form
    class="add"
    onsubmit={(event) => {
      event.preventDefault();
      void vm.add();
    }}
  >
    <TextField
      id="saving-new"
      label="Новое накопление"
      placeholder={PLACEHOLDERS.savingName}
      bind:value={vm.newName}
    />
    <Button
      type="submit"
      variant="secondary"
      loading={vm.adding}
      disabled={vm.newName.trim() === ''}>Добавить накопление</Button
    >
  </form>
  <p class="hint">
    Каждая сумма — обычный расход в категории накопления за выбранный месяц. Новый месяц начинается
    пустым.
  </p>
</section>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  /* Поле и обычная кнопка в одну строку: кнопка по высоте как поле, не растягивается. */
  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--sp-3);
  }
  .add :global(.field) {
    flex: 0 1 360px;
    width: auto;
  }
  .add :global(.btn) {
    flex: none;
  }
  .closed {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
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
  .empty,
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>

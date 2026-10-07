<script lang="ts">
  import {
    Button,
    MoneyInput,
    Segmented,
    Select,
    Sheet,
    StatusDot,
    TextField
  } from '$lib/components';
  import { categories } from '$lib/stores/categories.svelte';
  import { formatMoney, formatMonth, shiftMonth, today } from '$lib/format';
  import { getDebtsVm } from '../debts.svelte';

  const vm = getDebtsVm();

  const debt = $derived(vm.current);
  const isNew = $derived(vm.selected === 'new');
  const title = $derived(isNew ? 'Новый долг' : (debt?.lender ?? 'Долг'));

  const categoryOptions = $derived([
    { value: '', label: 'Без статьи' },
    ...categories.items
      .filter((c) => !c.archived)
      .map((c) => ({ value: String(c.id), label: c.name }))
  ]);

  const KINDS = [
    { value: 'equal', label: 'Равными частями' },
    { value: 'single', label: 'Одним платежом' }
  ] as const;

  const monthOptions = $derived(
    Array.from({ length: 36 }, (_, i) => {
      const month = shiftMonth(vm.draft.takenMonth, i);
      return { value: month, label: formatMonth(month) };
    })
  );

  const scheduleTotal = $derived(vm.schedule.reduce((sum, r) => sum + r.amount, 0));
</script>

<Sheet
  {title}
  width={460}
  onclose={() => {
    vm.close();
  }}
>
  <form
    id="debt-form"
    onsubmit={(event) => {
      event.preventDefault();
      void vm.save();
    }}
  >
    {#if vm.fromTransaction}
      <p class="from">
        Из траты «{vm.fromTransaction.title}»: {formatMoney(vm.fromTransaction.amount)}
      </p>
    {/if}

    <TextField id="debt-lender" label="У кого или что" required bind:value={vm.draft.lender} />

    {#if vm.fromTransaction === null && vm.scheduleEditable}
      <MoneyInput id="debt-amount" label="Сумма" bind:value={vm.draft.amount} />
    {:else if vm.fromTransaction === null && debt}
      <p class="fact">Сумма: <b class="num">{formatMoney(debt.amount)}</b></p>
    {/if}

    <p class="fact">Взят в месяце: <b>{formatMonth(vm.draft.takenMonth)}</b></p>

    <Select
      id="debt-category"
      label="Статья, на которую взят"
      options={categoryOptions}
      value={vm.draft.categoryId === null ? '' : String(vm.draft.categoryId)}
      onchange={(value) => {
        vm.draft.categoryId = value === '' ? null : Number(value);
      }}
    />
    <TextField
      id="debt-comment"
      label="Комментарий"
      required={false}
      bind:value={vm.draft.comment}
    />

    {#if debt}
      <section aria-labelledby="debt-schedule-title">
        <h3 id="debt-schedule-title">График погашения</h3>
        <ul class="payments">
          {#each debt.payments as p (p.id)}
            <li>
              <span class="month">{formatMonth(p.month)}</span>
              <span class="num amount">{formatMoney(p.amount)}</span>
              <span class="state">
                <StatusDot status={p.status === 'paid' ? 'paid' : 'planned'} />
                {p.status === 'paid' ? 'Оплачено' : 'План'}
              </span>
              <Button
                size="sm"
                variant="ghost"
                onclick={() => void vm.setPayment(p.id, p.status !== 'paid', today())}
              >
                {p.status === 'paid' ? 'Вернуть в план' : 'Отметить оплаченным'}
              </Button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if vm.scheduleEditable}
      <section aria-labelledby="debt-new-schedule-title">
        <h3 id="debt-new-schedule-title">{isNew ? 'График погашения' : 'Изменить график'}</h3>
        <Segmented
          label="Способ погашения"
          options={KINDS}
          value={vm.draft.kind}
          onchange={(value: 'equal' | 'single') => {
            vm.draft.kind = value;
          }}
        />
        {#if vm.draft.kind === 'equal'}
          <TextField id="debt-months" label="Количество месяцев" bind:value={vm.draft.months} />
        {:else}
          <Select
            id="debt-single-month"
            label="Месяц платежа"
            options={monthOptions}
            value={vm.draft.singleMonth}
            onchange={(value: string) => {
              vm.draft.singleMonth = value;
            }}
          />
        {/if}
        {#if vm.scheduleError}
          <p class="error" role="alert">{vm.scheduleError}</p>
        {:else if vm.schedule.length > 0}
          <ul class="payments preview">
            {#each vm.schedule as row (row.month)}
              <li>
                <span class="month">{formatMonth(row.month)}</span>
                <span class="num amount">{formatMoney(row.amount)}</span>
              </li>
            {/each}
          </ul>
          <p class="total">Сумма графика: <b class="num">{formatMoney(scheduleTotal)}</b></p>
        {/if}
      </section>
    {/if}

    {#if vm.fieldError}<p class="error" role="alert">{vm.fieldError}</p>{/if}
  </form>

  {#snippet footer()}
    {#if debt}
      <Button
        variant="danger"
        onclick={() => {
          void vm.remove(debt);
        }}>Удалить</Button
      >
    {/if}
    <Button
      variant="secondary"
      onclick={() => {
        vm.close();
      }}>Отмена</Button
    >
    <Button
      type="submit"
      form="debt-form"
      variant="primary"
      loading={vm.saving}
      disabled={!vm.canSave}
    >
      Сохранить
    </Button>
  {/snippet}
</Sheet>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  h3 {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-13);
    font-weight: var(--fw-semibold);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  .from,
  .fact,
  .total {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .fact b,
  .total b {
    color: var(--ink);
  }
  .payments {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .payments li {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) 0;
    border-bottom: 1px solid var(--line-2);
    font-size: var(--fs-13);
  }
  .payments.preview li {
    grid-template-columns: 1fr auto;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
    color: var(--muted);
  }
  .amount {
    font-weight: var(--fw-medium);
  }
  .error {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

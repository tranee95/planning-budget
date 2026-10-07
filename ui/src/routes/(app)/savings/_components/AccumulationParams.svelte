<script lang="ts">
  import { Button, Card, MoneyInput, Segmented, TextField } from '$lib/components';
  import { formatPercent } from '$lib/format';
  import { getSavingsVm } from '../savings.svelte';

  const vm = getSavingsVm();

  const KINDS = [
    { value: 'percent', label: '% от дохода' },
    { value: 'fixed', label: 'Сумма' }
  ] as const;

  const item = $derived(vm.selected);
  const title = $derived(item === null ? 'Параметры' : vm.nameOf(item.categoryId));
</script>

{#if item !== null}
  <Card {title}>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void vm.saveParams();
      }}
    >
      <div class="plan">
        <span class="lbl" id="plan-kind-label">План в месяц</span>
        <Segmented
          label="План в месяц"
          options={KINDS}
          value={vm.draft.planKind}
          onchange={(value: 'percent' | 'fixed') => {
            vm.draft.planKind = value;
          }}
        />
      </div>
      <div class="grid">
        {#if vm.draft.planKind === 'percent'}
          <TextField id="plan-percent" label="Доля дохода, %" bind:value={vm.draft.planPercent} />
        {:else}
          <MoneyInput id="plan-fixed" label="Сумма в месяц" bind:value={vm.draft.planFixed} />
        {/if}
        <TextField id="savings-rate" label="Ставка, % годовых" bind:value={vm.draft.rate} />
        <TextField id="savings-tax" label="Налог на купон, %" bind:value={vm.draft.tax} />
        <MoneyInput id="savings-balance" label="Стартовый баланс" bind:value={vm.draft.balance} />
        <label class="month">
          <span>Накопление с месяца</span>
          <input type="month" bind:value={vm.draft.month} required />
        </label>
      </div>
      {#if item.params.annualRateBp === 0}
        <p class="hint">Без процентов: баланс растёт только за счёт взносов.</p>
      {:else}
        <p class="hint">
          С учётом налога: {formatPercent(item.effectiveRateBp)} годовых. Купоны реинвестируются каждый
          месяц.
        </p>
      {/if}
      {#if vm.paramsError}<p class="error" role="alert">{vm.paramsError}</p>{/if}
      <div class="actions">
        <Button type="submit" variant="primary" loading={vm.saving}>Сохранить</Button>
      </div>
    </form>
  </Card>
{/if}

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  .plan {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .lbl {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--sp-3) var(--sp-4);
  }
  .month {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .month input {
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fs-14) var(--font-sans);
  }
  .hint,
  .error {
    margin: 0;
    font-size: var(--fs-13);
  }
  .hint {
    color: var(--muted);
  }
  .error {
    color: var(--unpl);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>

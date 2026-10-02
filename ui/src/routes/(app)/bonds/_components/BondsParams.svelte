<script lang="ts">
  import { Button, Card, MoneyInput, TextField } from '$lib/components';
  import { formatPercent } from '$lib/format';
  import { getBondsVm } from '../bonds.svelte';

  const vm = getBondsVm();

  // Ставка приходит числом или `null` (нечисловой результат): тогда пояснение не показывается.
  const rate = $derived(vm.data?.effectiveRate ?? null);
  const after = $derived(rate === null ? null : formatPercent(Math.round(rate * 10_000)));
</script>

<Card title="Параметры">
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void vm.saveParams();
    }}
  >
    <div class="grid">
      <TextField id="bonds-rate" label="Ставка, % годовых" bind:value={vm.draft.rate} />
      <TextField id="bonds-tax" label="Налог на купон, %" bind:value={vm.draft.tax} />
      <MoneyInput id="bonds-balance" label="Стартовый баланс" bind:value={vm.draft.balance} />
      <label class="month">
        <span>Накопление с месяца</span>
        <input type="month" bind:value={vm.draft.month} required />
      </label>
    </div>
    {#if after !== null}
      <p class="hint">С учётом налога: {after} годовых. Купоны реинвестируются каждый месяц.</p>
    {/if}
    {#if vm.paramsError}<p class="error" role="alert">{vm.paramsError}</p>{/if}
    <div class="actions">
      <Button type="submit" variant="primary" loading={vm.saving}>Сохранить</Button>
    </div>
  </form>
</Card>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
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

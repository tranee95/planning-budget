<script lang="ts">
  import { Button, Kbd, Modal, MoneyInput, Segmented } from '$lib/components';
  import type { IncomeStatusDto } from '$lib/api/bindings';
  import { monthRange } from '$lib/format';
  import { getIncomesVm } from '../incomes.svelte';

  const vm = getIncomesVm();
  const range = $derived(monthRange(vm.month));

  const STATUSES = [
    { value: 'received', label: 'Получено' },
    { value: 'expected', label: 'Ожидается' }
  ] as const;

  let amount = $state<number | null>(null);
  let sourceName = $state('');
  let date = $state('');
  let status = $state<IncomeStatusDto>('received');
  let saving = $state(false);

  const canSave = $derived(amount !== null && amount > 0 && sourceName.trim() !== '');

  async function save(): Promise<void> {
    if (!canSave || saving || amount === null) return;
    saving = true;
    try {
      await vm.create({ sourceName, amount, date: date === '' ? null : date, status });
    } finally {
      saving = false;
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Enter' || event.isComposing) return;
    if ((event.target as HTMLElement).tagName === 'BUTTON') return;
    event.preventDefault();
    void save();
  }
</script>

<Modal
  title="Новый доход"
  onclose={() => {
    vm.closeEditor();
  }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="form" onkeydown={onKeydown}>
    <MoneyInput id="income-amount" label="Сумма" size="lg" autofocus bind:value={amount} />
    <div class="row">
      <div class="group">
        <label class="lbl" for="income-source">Источник</label>
        <input id="income-source" class="field" autocomplete="off" bind:value={sourceName} />
      </div>
      <div class="group">
        <label class="lbl" for="income-date">Дата</label>
        <input
          id="income-date"
          class="field"
          type="date"
          min={range.first}
          max={range.last}
          bind:value={date}
        />
      </div>
    </div>
    <div class="group">
      <span class="lbl">Статус</span>
      <Segmented
        label="Статус дохода"
        options={STATUSES}
        value={status}
        onchange={(value: IncomeStatusDto) => {
          status = value;
        }}
      />
    </div>
  </div>

  {#snippet footer()}
    <Button
      size="lg"
      loading={saving}
      disabled={!canSave}
      onclick={() => {
        void save();
      }}>Сохранить <Kbd>↵</Kbd></Button
    >
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    min-width: 0;
  }
  .lbl {
    color: var(--muted);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: var(--sp-3);
  }
  .field {
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-14);
  }
  .field:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

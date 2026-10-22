<script lang="ts">
  import { untrack } from 'svelte';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { Button, DateField, Kbd, Modal, MoneyInput, Segmented } from '$lib/components';
  import type { IncomeStatusDto } from '$lib/api/bindings';
  import { getIncomesVm } from '../incomes.svelte';

  const vm = getIncomesVm();

  const STATUSES = [
    { value: 'received', label: 'Получено' },
    { value: 'expected', label: 'Ожидается' }
  ] as const;

  // Поля заполняются один раз при открытии: правка берёт значения дохода, создание — пустые.
  const editing = untrack(() => vm.editing);
  let amount = $state<number | null>(editing?.amount ?? null);
  let sourceName = $state(editing?.sourceName ?? '');
  let date = $state(editing?.date ?? '');
  let dateValid = $state(true);
  let status = $state<IncomeStatusDto>(editing?.status ?? 'received');
  let saving = $state(false);

  const canSave = $derived(amount !== null && amount > 0 && sourceName.trim() !== '' && dateValid);

  async function save(): Promise<void> {
    if (!canSave || saving || amount === null) return;
    saving = true;
    try {
      await vm.submit({ sourceName, amount, date: date === '' ? null : date, status });
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
  title={editing === null ? 'Новый доход' : 'Изменить доход'}
  closable
  onclose={() => {
    vm.closeEditor();
  }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="form" onkeydown={onKeydown}>
    <MoneyInput
      id="income-amount"
      label="Сумма"
      size="lg"
      autofocus
      placeholder={PLACEHOLDERS.incomeAmount}
      bind:value={amount}
    />
    <div class="row">
      <div class="group">
        <label class="lbl" for="income-source">Источник</label>
        <input
          id="income-source"
          class="field"
          autocomplete="off"
          placeholder={PLACEHOLDERS.income}
          bind:value={sourceName}
        />
      </div>
      <div class="group">
        <label class="lbl" for="income-date">Дата</label>
        <DateField
          id="income-date"
          bind:value={date}
          bind:valid={dateValid}
          describedby={dateValid ? undefined : 'income-date-error'}
        />
        {#if !dateValid}<span class="err" id="income-date-error" role="alert"
            >Укажите дату полностью: дд.мм.гггг</span
          >{/if}
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
    {#if editing !== null}
      <span class="delete-slot">
        <Button
          variant="danger-ghost"
          onclick={() => {
            void vm.remove(editing.id);
            vm.closeEditor();
          }}>Удалить</Button
        >
      </span>
    {/if}
    <Button
      variant="ghost"
      onclick={() => {
        vm.closeEditor();
      }}>Отмена</Button
    >
    <Button
      variant="primary"
      loading={saving}
      disabled={!canSave}
      onclick={() => {
        void save();
      }}>Сохранить <Kbd>↵</Kbd></Button
    >
  {/snippet}
</Modal>

<style>
  .delete-slot {
    margin-inline-end: auto;
  }
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
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  .err {
    color: var(--unpl);
    font-size: var(--fs-12);
  }
</style>

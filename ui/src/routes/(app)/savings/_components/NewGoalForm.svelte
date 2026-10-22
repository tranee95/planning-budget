<script lang="ts">
  import { Button } from '$lib/components';
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { getSavingsVm } from '../savings.svelte';
  import RowField from './RowField.svelte';

  const vm = getSavingsVm();

  let nameField = $state<HTMLInputElement>();

  $effect(() => {
    nameField?.focus();
  });

  function cancel(): void {
    vm.newOpen = false;
    vm.newName = '';
    vm.newPlan = null;
  }
</script>

<form
  class="form"
  aria-label="Новая цель"
  onsubmit={(event) => {
    event.preventDefault();
    void vm.add();
  }}
>
  <RowField
    label="Название цели"
    placeholder={PLACEHOLDERS.savingName}
    bind:value={vm.newName}
    bind:el={nameField}
    onescape={cancel}
  />
  <RowField
    label="План на месяц, ₽"
    kind="money"
    placeholder={PLACEHOLDERS.savingAmount}
    bind:value={vm.newPlan}
    onescape={cancel}
  />
  <div class="actions">
    <Button type="submit" variant="primary" loading={vm.adding} disabled={vm.newName.trim() === ''}
      >Создать</Button
    >
    <Button variant="ghost" onclick={cancel}>Отмена</Button>
  </div>
</form>

<style>
  .form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 200px auto;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
  }
  @container (max-width: 640px) {
    .form {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

<script lang="ts">
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import { onMount } from 'svelte';
  import { TextField } from '$lib/components';
  import { SavingsVm } from '../savings.svelte';

  const vm = new SavingsVm();

  onMount(() => {
    void vm.load();
  });
</script>

<form
  onsubmit={(e) => {
    // Enter в поле уже сохранил значение событием change: форма только не должна перезагружать страницу
    e.preventDefault();
  }}
>
  <p class="muted">
    Доля дохода, которую вы хотите откладывать. Сбережения ниже нижней границы помечаются как «не
    хватает». Изменения сохраняются сразу, когда поле теряет фокус.
  </p>
  <div class="row">
    <TextField
      id="sv-min"
      label="Нижняя граница"
      suffix="%"
      placeholder={PLACEHOLDERS.percent}
      bind:value={vm.min}
      disabled={!vm.loaded}
      onchange={() => void vm.save()}
    />
    <TextField
      id="sv-norm"
      label="Норма"
      suffix="%"
      placeholder={PLACEHOLDERS.percent}
      bind:value={vm.norm}
      disabled={!vm.loaded}
      onchange={() => void vm.save()}
    />
    <TextField
      id="sv-max"
      label="Верхняя граница"
      suffix="%"
      placeholder={PLACEHOLDERS.percent}
      bind:value={vm.max}
      disabled={!vm.loaded}
      onchange={() => void vm.save()}
    />
  </div>
  <p class="rule" class:broken={vm.error !== null}>
    Нижняя граница ≤ норма ≤ верхняя граница
    {#if vm.preview}· коридор {vm.preview}{/if}
  </p>
  {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sp-3);
  }
  .muted {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .rule {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .rule.broken {
    color: var(--danger);
  }
  .error {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, TextField } from '$lib/components';
  import { SavingsVm } from '../savings.svelte';

  const vm = new SavingsVm();

  onMount(() => {
    void vm.load();
  });
</script>

<form
  onsubmit={(e) => {
    e.preventDefault();
    void vm.save();
  }}
>
  <p class="muted">
    Доля дохода, которую вы хотите откладывать. Сбережения ниже нижней границы помечаются как «не
    хватает».
  </p>
  <div class="row">
    <TextField id="sv-min" label="Нижняя граница, %" bind:value={vm.min} disabled={!vm.loaded} />
    <TextField id="sv-norm" label="Норма, %" bind:value={vm.norm} disabled={!vm.loaded} />
    <TextField id="sv-max" label="Верхняя граница, %" bind:value={vm.max} disabled={!vm.loaded} />
  </div>
  {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}
  <Button type="submit" variant="primary" loading={vm.saving} disabled={!vm.loaded}
    >Сохранить</Button
  >
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
  .error {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
  form :global(button) {
    align-self: flex-start;
  }
</style>

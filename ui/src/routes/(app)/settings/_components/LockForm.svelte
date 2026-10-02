<script lang="ts">
  import { onMount } from 'svelte';
  import { Select, Switch } from '$lib/components';
  import { autolockOptions } from '$lib/autolock';
  import { LockSettingsVm } from '../lock-settings.svelte';

  const vm = new LockSettingsVm();

  onMount(() => {
    void vm.load();
  });
</script>

<div class="form">
  <Select
    id="autolock"
    label="Автоблокировка при бездействии"
    options={autolockOptions(vm.minutes)}
    value={vm.minutes}
    disabled={!vm.loaded}
    onchange={(value: string) => void vm.setMinutes(value)}
  />
  {#key vm.switchKey}
    <Switch
      checked={vm.lockOnMinimize}
      disabled={!vm.loaded}
      onchange={(on: boolean) => void vm.setLockOnMinimize(on)}
    >
      Блокировать при сворачивании окна
    </Switch>
  {/key}
  <p class="muted">
    Блокировка по таймеру считает и время сна компьютера. При сворачивании данные не видны в
    переключателе задач.
  </p>
  {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-4);
    margin-bottom: var(--sp-5);
  }
  .form :global(.field) {
    width: 100%;
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
</style>

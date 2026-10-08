<script lang="ts">
  import { Button } from '$lib/components';
  import type { LockVm } from '../lock.svelte';

  let { vm }: { vm: LockVm } = $props();
</script>

<div class="notice" role="alert">
  <p class="text">
    Не удалось перенести данные из прежней папки приложения. Прежние данные не изменены.
    {#if vm.migrationError === null}
      Повторите перенос до создания нового пароля.
    {/if}
  </p>
  {#if vm.migrationError !== null}
    <p class="text">{vm.migrationError}</p>
  {/if}
  <Button
    variant="secondary"
    size="sm"
    loading={vm.busy}
    onclick={() => {
      void vm.retryMigration();
    }}>Повторить перенос</Button
  >
</div>

<style>
  .notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
    width: 100%;
    margin-bottom: var(--sp-3);
    padding: var(--sp-3);
    border-radius: var(--r-sm);
    background: var(--debt-bg);
    color: var(--debt);
    font-size: var(--fs-13);
  }
  .text {
    margin: 0;
  }
</style>

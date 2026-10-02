<script lang="ts">
  import { Button, PasswordField } from '$lib/components';
  import type { LockVm } from '../lock.svelte';

  let { vm }: { vm: LockVm } = $props();

  $effect(() => (vm.waiting ? vm.watchRetry() : undefined));

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void vm.unlock();
  }
</script>

<form onsubmit={submit} class="form">
  <PasswordField
    id="password"
    size="lg"
    label="Пароль"
    bind:value={vm.password}
    autocomplete="current-password"
    error={vm.error}
    shake={vm.shake}
    reserveLines={2}
    autofocus
    hint={vm.waiting ? `Следующая попытка через ${String(vm.waitSeconds)} с` : null}
  />
  <Button type="submit" variant="primary" size="lg" loading={vm.busy} disabled={!vm.canUnlock}>
    Разблокировать
  </Button>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 14px;
    width: 100%;
  }
</style>

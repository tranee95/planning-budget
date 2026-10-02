<script lang="ts">
  import { Button, LinkButton, PasswordField, TextField } from '$lib/components';
  import StrengthMeter from '$lib/components/StrengthMeter.svelte';
  import type { LockVm } from '../lock.svelte';

  let { vm }: { vm: LockVm } = $props();

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void vm.resetByCode();
  }
</script>

<form onsubmit={submit} class="form">
  <TextField
    id="recovery-code"
    size="lg"
    label="Ключ восстановления"
    bind:value={vm.code}
    mono
    placeholder="XXXX-XXXX-XXXX-…"
    error={vm.codeError}
  />
  <PasswordField
    id="reset-password"
    size="lg"
    label="Новый пароль"
    bind:value={vm.resetPassword}
    autocomplete="new-password"
    error={vm.resetPasswordError}
  />
  <StrengthMeter password={vm.resetPassword} />
  <PasswordField
    id="reset-repeat"
    size="lg"
    label="Повторите новый пароль"
    bind:value={vm.resetRepeat}
    autocomplete="new-password"
    error={vm.resetMismatch ? 'Пароли не совпадают' : null}
  />
  <Button type="submit" variant="primary" size="lg" loading={vm.busy} disabled={!vm.canReset}>
    Задать новый пароль
  </Button>
  <LinkButton
    onclick={() => {
      vm.backToUnlock();
    }}>Вернуться ко входу</LinkButton
  >
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--sp-4);
    width: 100%;
  }
</style>

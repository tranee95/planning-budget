<script lang="ts">
  import { Button, PasswordField } from '$lib/components';
  import { MIN_PASSWORD_CHARS } from '$lib/security/password';
  import StrengthMeter from '$lib/components/StrengthMeter.svelte';
  import type { LockVm } from '../lock.svelte';

  let { vm }: { vm: LockVm } = $props();

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void vm.setup();
  }
</script>

<form onsubmit={submit} class="form">
  <PasswordField
    id="new-password"
    size="lg"
    label="Пароль"
    bind:value={vm.setupPassword}
    autocomplete="new-password"
    error={vm.setupTooShort ? `Не короче ${String(MIN_PASSWORD_CHARS)} символов` : null}
    hint={vm.setupTooShort
      ? null
      : `Не короче ${String(MIN_PASSWORD_CHARS)} символов. Подсказку к паролю мы не храним.`}
    reserveLines={1}
    autofocus
  />
  <!-- Место под индикатор держится всегда: форма не прыгает, когда он появляется при вводе. -->
  <div class="meter-slot"><StrengthMeter password={vm.setupPassword} /></div>
  <PasswordField
    id="repeat-password"
    size="lg"
    label="Повторите пароль"
    bind:value={vm.setupRepeat}
    autocomplete="new-password"
    error={vm.setupMismatch ? 'Пароли не совпадают' : vm.setupError}
    reserveLines={1}
  />
  <Button type="submit" variant="primary" size="lg" loading={vm.busy} disabled={!vm.canSetup}>
    Создать хранилище
  </Button>
</form>

<style>
  .meter-slot {
    min-height: 32px;
    margin-top: calc(-1 * var(--sp-4));
  }
  .form {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--sp-4);
    width: 100%;
  }
</style>

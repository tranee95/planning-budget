<script lang="ts">
  import { Button, PasswordField } from '$lib/components';
  import StrengthMeter from '$lib/components/StrengthMeter.svelte';
  import type { SettingsVm } from '../settings.svelte';

  let { vm }: { vm: SettingsVm } = $props();

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void vm.changePassword();
  }
</script>

<form onsubmit={submit} class="form">
  <h3>Смена пароля</h3>
  <PasswordField
    id="old-password"
    label="Текущий пароль"
    bind:value={vm.oldPassword}
    autocomplete="current-password"
    error={vm.oldError}
    shake={vm.shake}
  />
  <PasswordField
    id="change-password"
    label="Новый пароль"
    bind:value={vm.newPassword}
    autocomplete="new-password"
    error={vm.newError}
  />
  <StrengthMeter password={vm.newPassword} />
  <PasswordField
    id="change-repeat"
    label="Повторите новый пароль"
    bind:value={vm.repeat}
    autocomplete="new-password"
    error={vm.mismatch ? 'Пароли не совпадают' : null}
  />
  <p class="note">
    Ключ шифрования данных не меняется: старая резервная копия вместе со старым паролем по-прежнему
    откроет эти данные.
  </p>
  <p class="done" role="status">{vm.changed ? 'Пароль изменён.' : ''}</p>
  <Button type="submit" variant="primary" loading={vm.changing} disabled={!vm.canChange}
    >Сменить пароль</Button
  >
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-4);
  }
  h3 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .done {
    min-height: 1.4em;
    margin: 0;
    color: var(--paid);
    font-size: var(--fs-13);
  }
</style>

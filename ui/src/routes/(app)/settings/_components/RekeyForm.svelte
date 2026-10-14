<script lang="ts">
  import { Button, PasswordField } from '$lib/components';
  import type { SettingsVm } from '../settings.svelte';

  let { vm }: { vm: SettingsVm } = $props();

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void vm.rekey();
  }
</script>

<form onsubmit={submit} class="form">
  <h3>Перевыпуск ключа шифрования</h3>
  <p class="note">
    База перешифруется новым ключом, прежний ключ восстановления перестанет работать, а новый
    покажется сразу после перевыпуска: сохраните его.
  </p>
  <PasswordField
    id="rekey-password"
    label="Пароль для перевыпуска ключа"
    bind:value={vm.rekeyPassword}
    autocomplete="current-password"
    error={vm.rekeyError}
    shake={vm.rekeyShake}
  />
  <Button type="submit" loading={vm.rekeying} disabled={!vm.canRekey}>Перевыпустить ключ</Button>
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
</style>

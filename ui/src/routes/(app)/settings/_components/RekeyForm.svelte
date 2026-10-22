<script lang="ts">
  import { Button, Modal, PasswordField } from '$lib/components';
  import type { SettingsVm } from '../settings.svelte';

  let { vm }: { vm: SettingsVm } = $props();

  let confirming = $state(false);

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (vm.canRekey) confirming = true;
  }

  async function confirm(): Promise<void> {
    confirming = false;
    await vm.rekey();
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
  <Button type="submit" variant="danger" loading={vm.rekeying} disabled={!vm.canRekey}
    >Перевыпустить ключ</Button
  >
</form>

{#if confirming}
  <Modal
    title="Перевыпустить ключ?"
    width={460}
    onclose={() => {
      confirming = false;
    }}
  >
    <p class="ask">
      Прежний ключ восстановления перестанет работать. Новый покажется один раз сразу после
      перевыпуска: сохраните его, иначе при забытом пароле данные будет не открыть.
    </p>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          confirming = false;
        }}>Отмена</Button
      >
      <Button variant="danger" onclick={confirm}>Перевыпустить</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .ask {
    margin: 0;
    font-size: var(--fs-13);
    line-height: var(--lh-body);
  }
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

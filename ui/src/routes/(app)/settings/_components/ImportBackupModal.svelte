<script lang="ts">
  import { backupApi } from '$lib/api/backup';
  import { Button, Modal, PasswordField, TextField } from '$lib/components';
  import { errorField, errorText } from '$lib/i18n/errors';

  /** `fileName` — копия из каталога; без него файл выбирается в диалоге Rust. */
  let {
    fileName,
    onclose,
    ondone
  }: { fileName?: string; onclose: () => void; ondone: () => void } = $props();

  let label = $state('Из копии');
  let password = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let shake = $state(0);
  let labelError = $state<string | null>(null);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy || password === '' || label.trim() === '') return;
    busy = true;
    error = null;
    labelError = null;
    try {
      const done =
        fileName === undefined
          ? await backupApi.importPick(password, label)
          : (await backupApi.importNamed(fileName, password, label), true);
      // Диалог файла закрыт без выбора: окно остаётся открытым.
      if (done) ondone();
    } catch (e) {
      const text = errorText(e);
      if (errorField(e) === 'label') labelError = text;
      else error = text;
      shake += 1;
    } finally {
      password = '';
      busy = false;
    }
  }
</script>

<Modal title="Открыть копию как хранилище" width={460} {onclose}>
  <form id="import-backup" onsubmit={submit} class="form">
    <p class="note">
      Копия откроется отдельным хранилищем рядом с текущими, ничего не заменяя. Нужен пароль,
      который был у хранилища в момент создания копии.
    </p>
    {#if fileName === undefined}
      <p class="warn">
        Открывайте только копии, которые создали вы сами. Файл от другого человека может быть собран
        так, чтобы навредить при разборе.
      </p>
    {/if}
    <TextField id="import-label" label="Название хранилища" bind:value={label} error={labelError} />
    <PasswordField
      id="import-password"
      label="Пароль копии"
      bind:value={password}
      autocomplete="current-password"
      {error}
      {shake}
    />
  </form>
  {#snippet footer()}
    <Button variant="ghost" onclick={onclose}>Отмена</Button>
    <Button
      type="submit"
      form="import-backup"
      variant="primary"
      loading={busy}
      disabled={password === '' || label.trim() === ''}
      >{fileName === undefined ? 'Выбрать файл…' : 'Открыть'}</Button
    >
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .warn {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

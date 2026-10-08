<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import Download from '@lucide/svelte/icons/download';
  import { vaultApi } from '$lib/api/vault';
  import { Button, Checkbox } from '$lib/components';
  import { errorText } from '$lib/i18n/errors';
  import { copySecret, SECRET_CLIPBOARD_TTL_MS, type SecretCopy } from '$lib/security/clipboard';
  import { session } from '$lib/stores/session.svelte';

  type Props = { code: string };

  let { code }: Props = $props();

  let confirmed = $state(false);
  let busy = $state(false);
  let notice = $state<string | null>(null);
  let failure = $state<string | null>(null);

  const groups = $derived(code.split('-'));

  let copied: SecretCopy | null = null;

  // Код не должен пережить экран: при уходе с него (подтверждение, блокировка) буфер чистится.
  $effect(() => () => void copied?.clearNow());

  async function copy(): Promise<void> {
    failure = null;
    try {
      await copied?.clearNow();
      copied = await copySecret(code);
      notice = `Скопировано. Буфер очистится через ${String(SECRET_CLIPBOARD_TTL_MS / 1000)} с. Журнал буфера Windows (Win+V) мог сохранить копию: надёжнее записать код в файл.`;
    } catch {
      notice = null;
      failure = 'Не удалось скопировать. Выделите код и скопируйте вручную.';
    }
  }

  async function saveToFile(): Promise<void> {
    failure = null;
    try {
      const { saved } = await vaultApi.saveRecoveryCode(code);
      notice = saved ? 'Код записан в выбранный файл.' : null;
    } catch (e) {
      notice = null;
      failure = errorText(e);
    }
  }

  async function proceed(): Promise<void> {
    if (!confirmed || busy) return;
    busy = true;
    try {
      await copied?.clearNow();
      await session.acknowledgeRecovery();
    } finally {
      busy = false;
    }
  }
</script>

<div class="view">
  <div class="code" role="group" aria-label="Ключ восстановления">
    {#each groups as group, i (i)}<span>{group}</span>{/each}
  </div>
  <div class="actions">
    <Button onclick={copy}><Copy size={16} strokeWidth={2} aria-hidden="true" />Скопировать</Button>
    <Button onclick={saveToFile}
      ><Download size={16} strokeWidth={2} aria-hidden="true" />Сохранить в файл</Button
    >
  </div>
  <p class="status" role="status">{notice ?? ''}</p>
  {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
  <p class="warning">
    Код показывается один раз. Если потеряны и пароль, и код, данные восстановить нельзя.
  </p>
  <Checkbox bind:checked={confirmed}>Я сохранил код в надёжном месте</Checkbox>
  <Button variant="primary" size="lg" disabled={!confirmed} loading={busy} onclick={proceed}>
    Продолжить
  </Button>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    width: 100%;
  }
  /* Сетка 4 колонки: группы переносятся 4 + 3, а не оставляют последнюю группу одну в строке. */
  .code {
    display: grid;
    grid-template-columns: repeat(4, max-content);
    justify-content: center;
    gap: var(--sp-2) var(--sp-3);
    margin: 0;
    padding: var(--sp-5) var(--sp-4);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
    font-family: ui-monospace, 'Cascadia Mono', Consolas, monospace;
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
    letter-spacing: 0.06em;
    user-select: all;
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
  }
  .actions :global(.btn) {
    flex: 1;
  }
  .status,
  .failure,
  .warning {
    margin: 0;
    font-size: var(--fs-13);
  }
  .status {
    min-height: 1.4em;
    color: var(--paid);
  }
  .failure {
    color: var(--unpl);
  }
  .warning {
    color: var(--muted);
  }
</style>

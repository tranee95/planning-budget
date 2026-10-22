<script lang="ts">
  import { Button, Modal } from '$lib/components';
  import { updates } from '$lib/stores/updates.svelte';

  const offer = $derived(updates.offer);
</script>

{#if offer}
  <Modal
    title="Доступна версия {offer.version ?? ''}"
    width={520}
    onclose={() => {
      if (!updates.installing) updates.dismiss();
    }}
  >
    <p class="text">
      У вас версия {offer.currentVersion}. Приложение скачает обновление, проверит подпись, закроет
      хранилище и перезапустится. Данные не затрагиваются.
    </p>
    {#if offer.notes}
      <details class="changes">
        <summary>Что нового</summary>
        <p class="notes">{offer.notes}</p>
      </details>
    {/if}
    {#if updates.installing}
      <p class="progress" role="status" aria-live="polite">
        {updates.progress === null ? 'Загрузка…' : `Загрузка: ${String(updates.progress)} %`}
      </p>
    {/if}
    {#snippet footer()}
      <Button
        variant="ghost"
        disabled={updates.installing}
        onclick={() => {
          updates.dismiss();
        }}>Позже</Button
      >
      <Button
        variant="primary"
        loading={updates.installing}
        onclick={() => {
          void updates.install();
        }}>Установить и перезапустить</Button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .text,
  .notes,
  .progress {
    margin: 0 0 var(--sp-3);
    font-size: var(--fs-13);
    line-height: var(--lh-body);
  }
  .changes {
    margin-bottom: var(--sp-3);
    font-size: var(--fs-13);
  }
  .changes summary {
    cursor: pointer;
    color: var(--ink-2);
  }
  .changes .notes {
    margin-top: var(--sp-2);
    margin-bottom: 0;
  }
  .notes {
    max-height: 200px;
    overflow: auto;
    color: var(--muted);
    white-space: pre-wrap;
  }
  .progress {
    color: var(--muted);
  }
</style>

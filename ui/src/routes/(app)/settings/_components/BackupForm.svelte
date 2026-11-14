<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, Select, Switch } from '$lib/components';
  import { BackupVm, intervalOptions } from '../backup-settings.svelte';
  import ImportBackupModal from './ImportBackupModal.svelte';

  const vm = new BackupVm();
  let opening = $state<string | null>(null);

  onMount(() => {
    void vm.load();
  });

  const dateFormat = new Intl.DateTimeFormat('ru-RU', { dateStyle: 'medium', timeStyle: 'short' });

  function when(iso: string): string {
    return dateFormat.format(new Date(iso));
  }

  function size(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    return mb >= 1 ? `${mb.toFixed(1)} МБ` : `${String(Math.max(1, Math.round(bytes / 1024)))} КБ`;
  }
</script>

<div class="block">
  <h3>Резервные копии</h3>
  <p class="note">
    Копия зашифрована так же, как база: открыть её можно только паролем хранилища на момент
    создания. Хранятся последние 7 копий каждого хранилища.
  </p>

  {#key vm.switchKey}
    <Switch checked={vm.auto} disabled={!vm.loaded} onchange={(on: boolean) => void vm.setAuto(on)}>
      Создавать копии автоматически при входе
    </Switch>
  {/key}
  <Select
    id="backup-interval"
    label="Как часто"
    options={intervalOptions(vm.intervalDays)}
    value={vm.intervalDays}
    disabled={!vm.loaded || !vm.auto}
    onchange={(value: string) => void vm.setInterval(value)}
  />

  <div class="dir">
    <span class="meta">Каталог копий</span>
    <code class="path">{vm.status?.dir ?? '…'}</code>
    <div class="actions">
      <Button size="sm" disabled={!vm.loaded} onclick={() => void vm.pickDir()}>Выбрать…</Button>
      {#if vm.status && !vm.status.dirIsDefault}
        <Button size="sm" variant="ghost" onclick={() => void vm.resetDir()}>
          Вернуть по умолчанию
        </Button>
      {/if}
    </div>
  </div>

  <Button
    variant="primary"
    loading={vm.creating}
    disabled={!vm.loaded}
    onclick={() => void vm.create()}
  >
    Создать копию сейчас
  </Button>

  {#if vm.status}
    {#if vm.status.items.length === 0}
      <p class="note">Копий этого хранилища в каталоге пока нет.</p>
    {:else}
      <ul class="list" aria-label="Копии активного хранилища">
        {#each vm.status.items as item (item.fileName)}
          <li class="row">
            <div class="what">
              <span class="label">{when(item.createdAt)}</span>
              <span class="meta">{size(item.sizeBytes)}</span>
            </div>
            <Button
              size="sm"
              variant="ghost"
              onclick={() => {
                opening = item.fileName;
              }}>Открыть как хранилище</Button
            >
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
  {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}
</div>

{#if opening !== null}
  <ImportBackupModal
    fileName={opening}
    onclose={() => {
      opening = null;
    }}
    ondone={() => {
      opening = null;
    }}
  />
{/if}

<style>
  .block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-4);
  }
  .block :global(.field) {
    width: 100%;
  }
  h3 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .note,
  .meta {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .dir {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    width: 100%;
  }
  .path {
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    font-size: var(--fs-12);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }
  .list {
    width: 100%;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    padding-block: var(--sp-2);
    border-bottom: 1px solid var(--line);
  }
  .what {
    display: flex;
    flex-direction: column;
  }
  .label {
    font-weight: var(--fw-medium);
  }
  .error {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

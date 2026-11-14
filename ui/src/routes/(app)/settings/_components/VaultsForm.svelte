<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, Modal, TextField } from '$lib/components';
  import { VaultsVm } from '../backup-settings.svelte';
  import ImportBackupModal from './ImportBackupModal.svelte';

  const vm = new VaultsVm();

  // add — новое пустое хранилище; rename — переименование дополнительного.
  let naming = $state<{ kind: 'add' } | { kind: 'rename'; id: string } | null>(null);
  let name = $state('');
  let importing = $state(false);

  onMount(() => {
    void vm.load();
  });

  function startAdd(): void {
    name = '';
    naming = { kind: 'add' };
  }

  function startRename(id: string, label: string): void {
    name = label;
    naming = { kind: 'rename', id };
  }

  async function saveName(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const target = naming;
    if (target === null || name.trim() === '') return;
    const ok = target.kind === 'add' ? await vm.add(name) : await vm.rename(target.id, name);
    if (ok) naming = null;
  }
</script>

<div class="block">
  <h3>Хранилища</h3>
  <p class="note">
    У каждого хранилища свои база, пароль и ключ восстановления. После переключения нужно войти с
    паролем выбранного хранилища.
  </p>
  <ul class="list" aria-label="Хранилища">
    {#each vm.items as item (item.id)}
      <li class="row">
        <div class="what">
          <span class="label">{item.label}</span>
          <span class="meta">
            {#if item.active}Активно{:else if !item.exists}Не создано: пароль задаётся при первом
              входе{/if}
          </span>
        </div>
        <div class="actions">
          {#if !item.active}
            <Button size="sm" onclick={() => void vm.switchTo(item.id)}>Переключить</Button>
          {/if}
          {#if item.id !== 'main'}
            <Button
              size="sm"
              variant="ghost"
              onclick={() => {
                startRename(item.id, item.label);
              }}
            >
              Переименовать
            </Button>
          {/if}
          {#if item.id !== 'main' && !item.active && !item.exists}
            <Button size="sm" variant="danger-ghost" onclick={() => void vm.remove(item.id)}>
              Убрать
            </Button>
          {/if}
        </div>
      </li>
    {/each}
  </ul>
  <div class="actions">
    <Button onclick={startAdd}>Добавить хранилище</Button>
    <Button
      onclick={() => {
        importing = true;
      }}>Открыть копию из файла…</Button
    >
  </div>
  {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}
</div>

{#if naming !== null}
  <Modal
    title={naming.kind === 'add' ? 'Новое хранилище' : 'Название хранилища'}
    width={420}
    onclose={() => {
      naming = null;
    }}
  >
    <form id="vault-name" onsubmit={saveName}>
      <TextField id="vault-label" label="Название" bind:value={name} error={vm.error} />
    </form>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          naming = null;
        }}>Отмена</Button
      >
      <Button type="submit" form="vault-name" variant="primary" disabled={name.trim() === ''}
        >{naming?.kind === 'add' ? 'Создать' : 'Сохранить'}</Button
      >
    {/snippet}
  </Modal>
{/if}

{#if importing}
  <ImportBackupModal
    onclose={() => {
      importing = false;
    }}
    ondone={() => {
      importing = false;
    }}
  />
{/if}

<style>
  .block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-4);
    margin-bottom: var(--sp-5);
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
    flex-wrap: wrap;
    padding-block: var(--sp-2);
    border-bottom: 1px solid var(--line);
  }
  .what {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    font-weight: var(--fw-medium);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }
  .error {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

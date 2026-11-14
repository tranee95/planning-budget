<script lang="ts">
  import { onMount } from 'svelte';
  import type { TagUsageDto } from '$lib/api/bindings';
  import { Button, Modal, TextField } from '$lib/components';
  import { TagsVm, deleteHint, renameHint, usesText } from '../tags-settings.svelte';

  const vm = new TagsVm();

  let renaming = $state<TagUsageDto | null>(null);
  let deleting = $state<TagUsageDto | null>(null);
  let name = $state('');

  onMount(() => {
    void vm.load();
  });

  /** Перед окном счётчики перечитываются: траты и запросы могли измениться с открытия настроек. */
  async function startRename(id: number): Promise<void> {
    vm.clearError();
    await vm.load();
    const tag = vm.items.find((t) => t.id === id);
    if (tag === undefined) return;
    name = tag.name;
    renaming = tag;
  }

  async function startDelete(id: number): Promise<void> {
    vm.clearError();
    await vm.load();
    deleting = vm.items.find((t) => t.id === id) ?? null;
  }

  async function saveName(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const target = renaming;
    if (target === null || name.trim() === '') return;
    if (await vm.rename(target.id, name)) renaming = null;
  }

  async function confirmDelete(): Promise<void> {
    const target = deleting;
    if (target === null) return;
    if (await vm.remove(target)) deleting = null;
  }
</script>

<div class="block">
  <h3>Теги</h3>
  <p class="note">
    Теги ставятся на траты на экране «Расходы» и работают в поиске (<code>#тег</code>) и в
    «Аналитике». Новый тег создаётся там же, при выборе тегов траты.
  </p>
  {#if vm.loaded && vm.items.length === 0}
    <p class="note">Тегов пока нет.</p>
  {:else}
    <ul class="list" aria-label="Теги">
      {#each vm.items as tag (tag.id)}
        <li class="row">
          <div class="what">
            <span class="label">{tag.name}</span>
            <span class="meta">{usesText(tag.uses)}</span>
          </div>
          <div class="actions">
            <Button
              size="sm"
              variant="ghost"
              aria-label="Переименовать тег «{tag.name}»"
              onclick={() => {
                void startRename(tag.id);
              }}>Переименовать</Button
            >
            <Button
              size="sm"
              variant="danger-ghost"
              aria-label="Удалить тег «{tag.name}»"
              onclick={() => {
                void startDelete(tag.id);
              }}>Удалить</Button
            >
          </div>
        </li>
      {/each}
    </ul>
  {/if}
  {#if vm.error && renaming === null && deleting === null}
    <p class="error" role="alert">{vm.error}</p>
  {/if}
</div>

{#if renaming !== null}
  <Modal
    title="Название тега"
    width={420}
    onclose={() => {
      renaming = null;
    }}
  >
    <form id="tag-name" onsubmit={saveName}>
      <TextField id="tag-label" label="Название" bind:value={name} error={vm.error} />
      {#if renameHint(renaming) !== ''}<p class="note">{renameHint(renaming)}</p>{/if}
    </form>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          renaming = null;
        }}>Отмена</Button
      >
      <Button type="submit" form="tag-name" variant="primary" disabled={name.trim() === ''}
        >Сохранить</Button
      >
    {/snippet}
  </Modal>
{/if}

{#if deleting !== null}
  <Modal
    title="Удалить тег?"
    width={420}
    onclose={() => {
      deleting = null;
    }}
  >
    <p class="confirm"><strong>{deleting.name}</strong>. {deleteHint(deleting)}</p>
    {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          deleting = null;
        }}>Отмена</Button
      >
      <Button variant="danger" onclick={() => void confirmDelete()}>Удалить</Button>
    {/snippet}
  </Modal>
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
  .confirm {
    margin: 0;
    font-size: var(--fs-14);
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

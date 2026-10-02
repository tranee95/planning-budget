<script lang="ts">
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import type { TransactionDto } from '$lib/api/bindings';
  import { Button, Modal } from '$lib/components';
  import { errorText } from '$lib/i18n/errors';
  import { tags } from '$lib/stores/tags.svelte';
  import { toasts } from '$lib/stores/toasts.svelte';

  type Props = {
    row: TransactionDto;
    onsave: (tagIds: number[]) => void;
    onclose: () => void;
  };

  let { row, onsave, onclose }: Props = $props();

  const chosen = new SvelteSet<number>(untrack(() => row.tagIds));
  let draft = $state('');

  function toggle(id: number): void {
    if (chosen.has(id)) chosen.delete(id);
    else chosen.add(id);
  }

  async function create(): Promise<void> {
    const name = draft.trim();
    if (name === '') return;
    try {
      const tag = await tags.create(name);
      chosen.add(tag.id);
      draft = '';
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }
</script>

<Modal title="Теги: {row.title}" width={420} {onclose}>
  {#if tags.items.length === 0}
    <p class="none">Тегов пока нет. Добавьте первый ниже.</p>
  {:else}
    <ul class="list" aria-label="Теги">
      {#each tags.items as tag (tag.id)}
        <li>
          <label>
            <input
              type="checkbox"
              checked={chosen.has(tag.id)}
              onchange={() => {
                toggle(tag.id);
              }}
            />
            {tag.name}
          </label>
        </li>
      {/each}
    </ul>
  {/if}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="new"
    onkeydown={(event) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        event.stopPropagation();
        void create();
      }
    }}
  >
    <input aria-label="Новый тег" placeholder="Новый тег" autocomplete="off" bind:value={draft} />
    <Button
      variant="secondary"
      size="sm"
      disabled={draft.trim() === ''}
      onclick={() => void create()}>Добавить</Button
    >
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={onclose}>Отмена</Button>
    <Button
      onclick={() => {
        onsave([...chosen]);
      }}>Сохранить</Button
    >
  {/snippet}
</Modal>

<style>
  .none {
    margin: 0 0 var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .list {
    display: flex;
    flex-direction: column;
    max-height: 260px;
    margin: 0 0 var(--sp-3);
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }
  label {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    min-height: 32px;
    cursor: pointer;
  }
  .new {
    display: flex;
    gap: var(--sp-2);
  }
  .new input {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }
</style>

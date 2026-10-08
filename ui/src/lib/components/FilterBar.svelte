<script lang="ts">
  import { rise } from '$lib/motion';
  import Search from '@lucide/svelte/icons/search';
  import X from '@lucide/svelte/icons/x';
  import { errorText } from '$lib/i18n/errors';
  import { formatMoney } from '$lib/format';
  import type { FilterScreenDto } from '$lib/api/bindings';
  import type { QueryFilter } from '$lib/filters/query-filter.svelte';
  import { savedFilters } from '$lib/stores/saved-filters.svelte';
  import { toasts } from '$lib/stores/toasts.svelte';
  import { onMount } from 'svelte';
  import Button from './Button.svelte';
  import PaletteChips from './PaletteChips.svelte';

  type Props = {
    filter: QueryFilter;
    /** Что за таблица: попадает в подпись поля и в строку «Найдено». */
    subject: 'трат' | 'доходов';
    /** Экран: сохранённые фильтры показываются и создаются по экранам. */
    screen: FilterScreenDto;
    placeholder: string;
  };

  let { filter, subject, screen, placeholder }: Props = $props();

  const fieldId = $props.id();
  const screenFilters = $derived(savedFilters.items.filter((f) => f.screen === screen));
  let naming = $state(false);
  let name = $state('');

  onMount(() => {
    void savedFilters.ensure().catch(() => {
      // Без сохранённых фильтров панель работает: строка запроса важнее.
    });
  });

  async function save(): Promise<void> {
    const text = filter.applied;
    if (text === '' || name.trim() === '') return;
    try {
      await savedFilters.save(name, text, screen);
      naming = false;
      name = '';
      toasts.push({ message: 'Фильтр сохранён' });
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }

  async function remove(id: number): Promise<void> {
    try {
      await savedFilters.remove(id);
    } catch (e) {
      toasts.push({ kind: 'error', message: errorText(e) });
    }
  }
</script>

<div class="bar" role="search" in:rise>
  <div class="field">
    <Search size={16} aria-hidden="true" />
    <input
      id={fieldId}
      type="text"
      aria-label="Фильтр {subject}"
      {placeholder}
      autocomplete="off"
      spellcheck="false"
      value={filter.text}
      oninput={(event) => {
        filter.type(event.currentTarget.value);
      }}
    />
    {#if filter.text !== ''}
      <button
        type="button"
        class="clear"
        aria-label="Очистить фильтр"
        onclick={() => {
          filter.clear();
        }}
      >
        <X size={14} aria-hidden="true" />
      </button>
    {/if}
  </div>
  {#if filter.active && !naming}
    <Button
      variant="secondary"
      size="sm"
      onclick={() => {
        naming = true;
      }}>Сохранить фильтр</Button
    >
  {/if}
  {#if naming}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="naming"
      onkeydown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          void save();
        } else if (event.key === 'Escape') {
          event.stopPropagation();
          naming = false;
        }
      }}
    >
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="name"
        aria-label="Название фильтра"
        placeholder="Название"
        maxlength="60"
        autocomplete="off"
        autofocus
        bind:value={name}
      />
      <Button size="sm" disabled={name.trim() === ''} onclick={() => void save()}>Сохранить</Button>
    </div>
  {/if}
</div>

{#if filter.meta}
  <PaletteChips
    query={filter.meta.query}
    spans={filter.meta.spans}
    hints={filter.meta.hints}
    onremove={(next: string) => {
      filter.set(next);
    }}
  />
{/if}
{#if filter.active && filter.meta}
  <p class="found" role="status">
    Найдено {subject}: <b>{filter.meta.total}</b> на <b>{formatMoney(filter.meta.sum)}</b>
    {#if filter.meta.truncated}· показаны не все: уточните запрос{/if}
  </p>
{/if}
{#if screenFilters.length > 0}
  <ul class="saved" aria-label="Сохранённые фильтры">
    {#each screenFilters as item (item.id)}
      <li>
        <button
          type="button"
          class="apply"
          title={item.query}
          onclick={() => {
            filter.set(item.query);
          }}>{item.name}</button
        >
        <button
          type="button"
          class="drop"
          aria-label="Удалить фильтр «{item.name}»"
          onclick={() => void remove(item.id)}
        >
          <X size={12} aria-hidden="true" />
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
  }
  .field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--sp-2);
    min-width: 260px;
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--muted);
  }
  .field:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: var(--fs-14) var(--font-sans);
    outline: none;
  }
  .clear,
  .drop {
    display: grid;
    flex: none;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: var(--r-full);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .clear:hover,
  .drop:hover {
    background: var(--line-2);
    color: var(--ink);
  }
  .naming {
    display: flex;
    gap: var(--sp-2);
  }
  .name {
    flex: none;
    width: 180px;
    height: 32px;
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
  }
  .found {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .saved {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .saved li {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding-inline: var(--sp-3) var(--sp-1);
    border: 1px solid var(--line);
    border-radius: var(--r-full);
    background: var(--surface);
  }
  .apply {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: var(--fs-12) var(--font-sans);
    font-weight: var(--fw-medium);
    cursor: pointer;
  }
</style>

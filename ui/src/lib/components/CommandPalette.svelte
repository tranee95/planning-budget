<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import { pop, scrimFade } from '$lib/motion';
  import { buildGroups, type PaletteTarget } from '$lib/palette/entries';
  import type { PaletteCommand } from '$lib/palette/filter';
  import { completeToken } from '$lib/palette/query';
  import { PaletteSearch, recentQueries, type SearchFn } from '$lib/palette/search.svelte';
  import { savedFilters } from '$lib/stores/saved-filters.svelte';
  import Kbd from './Kbd.svelte';
  import PaletteChips from './PaletteChips.svelte';
  import PaletteRow from './PaletteRow.svelte';
  import { trapFocus } from './overlay';

  type Props = {
    commands: readonly PaletteCommand[];
    onclose: () => void;
    /** Поиск по данным; без него палитра ищет только команды. */
    search?: SearchFn;
    /** Переход к найденной записи, категории или месяцу. */
    onopen?: (target: PaletteTarget) => void;
  };

  let { commands, onclose, search, onopen }: Props = $props();

  const listId = $props.id();
  const finder = new PaletteSearch(async (q, id) => {
    if (!search) throw new Error('search is not configured');
    return search(q, id);
  });
  let active = $state(0);

  $effect(() => {
    if (search) {
      void savedFilters.ensure().catch(() => {
        // Без сохранённых фильтров палитра работает: поиск и команды важнее.
      });
    }
    return () => {
      finder.dispose();
    };
  });

  const groups = $derived(
    buildGroups(
      finder.query,
      finder.result?.data ?? null,
      commands,
      recentQueries.items,
      savedFilters.items,
      finder.result?.query ?? finder.query
    )
  );
  const entries = $derived(groups.flatMap((g) => g.entries));
  const activeIndex = $derived(Math.min(active, Math.max(entries.length - 1, 0)));
  const optionId = (key: string): string => `${listId}-${key}`;
  const activeEntry = $derived(entries[activeIndex]);

  function setQuery(next: string): void {
    active = 0;
    if (search) finder.set(next);
    else finder.query = next;
  }

  function pick(target: PaletteTarget | undefined): void {
    if (!target) return;
    if (target.type === 'recent') {
      setQuery(target.query);
      return;
    }
    if (target.type === 'command') {
      onclose();
      target.command.run();
      return;
    }
    // Результат ещё для прошлого ввода: Enter не должен открыть запись, которой уже нет в списке.
    if (finder.pending) return;
    recentQueries.remember(finder.result?.query ?? finder.query);
    onclose();
    onopen?.(target);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      active = (activeIndex + step + entries.length) % Math.max(entries.length, 1);
    } else if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      // Ctrl/⌘ ↵: все найденные записи таблицей; траты важнее доходов.
      const table = entries.find((e) => e.target.type === 'table' && e.key.startsWith('table:'));
      if (!table) return;
      event.preventDefault();
      pick(table.target);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      pick(activeEntry?.target);
    } else if (event.key === 'Tab' && !event.shiftKey) {
      const completed = completeToken(finder.query);
      if (completed === null) return;
      event.preventDefault();
      event.stopPropagation();
      setQuery(completed);
    }
  }
</script>

<div class="layer">
  <button
    type="button"
    class="scrim"
    tabindex="-1"
    aria-label="Закрыть"
    onclick={onclose}
    transition:scrimFade
  ></button>
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label="Поиск и команды"
    use:trapFocus={onclose}
    transition:pop
  >
    <div class="search">
      <Search size={18} aria-hidden="true" />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls={listId}
        aria-activedescendant={activeEntry ? optionId(activeEntry.key) : undefined}
        aria-label="Поиск"
        placeholder="Поиск и команды"
        autocomplete="off"
        spellcheck="false"
        autofocus
        value={finder.query}
        oninput={(event) => {
          setQuery(event.currentTarget.value);
        }}
        onkeydown={onKeydown}
      />
      <Kbd>Esc</Kbd>
    </div>
    {#if finder.result}
      <PaletteChips
        query={finder.result.query}
        spans={finder.result.data.spans}
        hints={finder.result.data.hints}
        onremove={setQuery}
      />
    {/if}
    <div id={listId} class="results" role="listbox" aria-label="Результаты">
      {#each groups as group (group.id)}
        <section role="group" aria-labelledby={optionId(`h-${group.id}`)}>
          <h2 id={optionId(`h-${group.id}`)}>{group.title}</h2>
          <ul role="presentation">
            {#each group.entries as entry (entry.key)}
              <PaletteRow
                id={optionId(entry.key)}
                {entry}
                active={entry.key === activeEntry?.key}
                onpick={() => {
                  pick(entry.target);
                }}
                onhover={() => {
                  active = entries.indexOf(entry);
                }}
              />
            {/each}
          </ul>
        </section>
      {:else}
        <p class="none" role="presentation">
          Ничего не найдено. Фильтры: статус:оплачено, сумма&gt;5000, период:июн..сен, #тег
        </p>
      {/each}
    </div>
    <footer>
      <span><Kbd>↑↓</Kbd> выбрать</span>
      <span><Kbd>↵</Kbd> открыть</span>
      <span><Kbd>Tab</Kbd> добавить фильтр</span>
      <span class="spacer"></span>
      <span>сумма&gt;5000 · период:июн..сен · #подарки</span>
    </footer>
  </div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: var(--z-palette);
    display: flex;
    justify-content: center;
    padding-top: 12vh;
  }
  .scrim {
    position: absolute;
    inset: 0;
    border: 0;
    background: var(--scrim);
    cursor: default;
  }
  .panel {
    position: relative;
    align-self: flex-start;
    width: min(680px, calc(100% - var(--sp-8)));
    overflow: hidden;
    border-radius: var(--r-xl);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding-inline: var(--sp-5);
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }
  input {
    flex: 1;
    height: 60px;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: var(--fw-medium) var(--fs-17) var(--font-sans);
    outline: none;
  }
  .results {
    max-height: min(420px, 55vh);
    padding: var(--sp-2);
    overflow-y: auto;
  }
  h2 {
    margin: var(--sp-2) var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .none {
    margin: 0;
    padding: var(--sp-6) var(--sp-4);
    color: var(--muted);
    font-size: var(--fs-13);
    text-align: center;
  }
  footer {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    height: 40px;
    padding-inline: var(--sp-5);
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .spacer {
    flex: 1;
  }
</style>

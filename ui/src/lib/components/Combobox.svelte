<script lang="ts">
  import { filterCommands } from '$lib/palette/filter';
  import { placeList } from './popover';

  type Option = { value: string; label: string };

  type Props = {
    id: string;
    label: string;
    options: readonly Option[];
    /** `value` выбранного варианта или `null`. */
    value: string | null;
    placeholder?: string;
    onchange: (value: string) => void;
  };

  let { id, label, options, value, placeholder, onchange }: Props = $props();

  const listId = $props.id();
  let open = $state(false);
  let query = $state('');
  let active = $state(0);
  let box = $state<HTMLDivElement>();
  let listStyle = $state('');

  // Список лежит в `position: fixed` и открывается туда, где есть место (у нижнего края окна —
  // вверх): так его не обрезает прокручиваемая область модалки или панели.
  function place(): void {
    if (!box) return;
    const zoom = parseFloat(document.documentElement.style.zoom) || 1;
    const r = box.getBoundingClientRect();
    const viewport = window.innerHeight;
    const result = placeList({
      triggerTop: r.top,
      triggerBottom: r.bottom,
      viewport,
      desired: 240 * zoom
    });
    const gap = 4;
    const common = `left:${String(r.left / zoom)}px;width:${String(r.width / zoom)}px;max-height:${String(result.maxHeight / zoom)}px;`;
    listStyle = result.up
      ? `${common}bottom:${String((viewport - r.top + gap) / zoom)}px;`
      : `${common}top:${String((r.bottom + gap) / zoom)}px;`;
  }
  function show(): void {
    place();
    open = true;
  }

  const selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? '');
  // Поиск общий с палитрой: ё = е, регистр не важен, начало слова выше.
  const results = $derived(
    filterCommands(
      options.map((o) => ({ id: o.value, label: o.label, group: '', run: () => {} })),
      query
    )
  );
  const activeIndex = $derived(Math.min(active, Math.max(results.length - 1, 0)));
  const optionId = (i: number): string => `${listId}-${String(i)}`;

  function choose(optionValue: string | undefined): void {
    if (optionValue === undefined) return;
    onchange(optionValue);
    open = false;
    query = '';
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) {
        show();
        return;
      }
      const step = event.key === 'ArrowDown' ? 1 : -1;
      active = (activeIndex + step + results.length) % Math.max(results.length, 1);
    } else if (event.key === 'Enter' && open) {
      event.preventDefault();
      choose(results[activeIndex]?.id);
    } else if (event.key === 'Escape' && open) {
      event.stopPropagation();
      open = false;
      query = '';
    }
  }
</script>

<svelte:window
  onresize={() => {
    if (open) place();
  }}
  onscrollcapture={() => {
    if (open) place();
  }}
/>

<div class="field">
  <label for={id}>{label}</label>
  <div class="box" bind:this={box}>
    <input
      {id}
      type="text"
      role="combobox"
      aria-expanded={open}
      data-escape-closes-list={open}
      aria-controls={listId}
      aria-autocomplete="list"
      aria-activedescendant={open && results.length ? optionId(activeIndex) : undefined}
      autocomplete="off"
      {placeholder}
      value={open ? query : selectedLabel}
      onclick={show}
      onblur={() => {
        open = false;
        query = '';
      }}
      oninput={(event) => {
        query = event.currentTarget.value;
        active = 0;
        show();
      }}
      onkeydown={onKeydown}
    />
    {#if open}
      <ul id={listId} role="listbox" aria-label={label} style={listStyle}>
        {#each results as option, i (option.id)}
          <li
            id={optionId(i)}
            role="option"
            aria-selected={option.id === value}
            class:active={i === activeIndex}
            onpointermove={() => {
              active = i;
            }}
            onpointerdown={(event) => {
              // Не даём полю потерять фокус раньше выбора.
              event.preventDefault();
              choose(option.id);
            }}
          >
            {option.label}
          </li>
        {:else}
          <li class="none" role="presentation">Ничего не найдено</li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }
  label {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .box {
    position: relative;
  }
  input {
    width: 100%;
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fs-14) var(--font-sans);
  }
  input:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  ul {
    position: fixed;
    z-index: var(--z-modal);
    margin: 0;
    padding: var(--sp-1);
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    box-shadow: var(--shadow);
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    min-height: var(--h-control-sm);
    padding-inline: var(--sp-3);
    border-radius: var(--r-xs);
    font-size: var(--fs-14);
    cursor: pointer;
  }
  li.active {
    background: var(--accent-soft);
  }
  .none {
    color: var(--muted);
    cursor: default;
  }
</style>

<script lang="ts" generics="T extends string">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Check from '@lucide/svelte/icons/check';
  import { placeList } from './popover';

  type Props = {
    id: string;
    label: string;
    options: readonly { value: T; label: string }[];
    value: T;
    onchange: (value: T) => void;
    disabled?: boolean;
    /** Подпись остаётся для скринридеров, но не занимает места над полем (в строке с кнопками). */
    hideLabel?: boolean;
    /** Значения, которые сейчас выбрать нельзя, и причина (показывается подсказкой). */
    blocked?: (value: T) => string | null;
  };

  let {
    id,
    label,
    options,
    value,
    onchange,
    disabled = false,
    hideLabel = false,
    blocked
  }: Props = $props();

  // Свой список вместо нативного <select>: единый вид, клавиатура и aria (WAI-ARIA «select-only combobox»).
  const listId = $props.id();
  let open = $state(false);
  let active = $state(0);
  let box = $state<HTMLDivElement>();
  let listStyle = $state('');
  let typed = '';
  let typedTimer: ReturnType<typeof setTimeout> | undefined;

  const selectedIndex = $derived(
    Math.max(
      options.findIndex((o) => o.value === value),
      0
    )
  );
  const selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? '');
  const optionId = (i: number): string => `${listId}-${String(i)}`;
  const isBlocked = (i: number): boolean => {
    const option = options[i];
    return option !== undefined && blocked?.(option.value) != null;
  };

  function place(): void {
    if (!box) return;
    const zoom = parseFloat(document.documentElement.style.zoom) || 1;
    const r = box.getBoundingClientRect();
    const viewport = window.innerHeight;
    const result = placeList({
      triggerTop: r.top,
      triggerBottom: r.bottom,
      viewport,
      desired: Math.min(options.length * 36 + 8, 280) * zoom
    });
    const gap = 4;
    const common = `left:${String(r.left / zoom)}px;min-width:${String(r.width / zoom)}px;max-height:${String(result.maxHeight / zoom)}px;`;
    listStyle = result.up
      ? `${common}bottom:${String((viewport - r.top + gap) / zoom)}px;`
      : `${common}top:${String((r.bottom + gap) / zoom)}px;`;
  }

  function show(): void {
    if (disabled) return;
    place();
    active = selectedIndex;
    open = true;
  }

  function close(): void {
    open = false;
  }

  function choose(i: number): void {
    const option = options[i];
    if (option === undefined || isBlocked(i)) return;
    onchange(option.value);
    close();
  }

  /** Следующий доступный пункт в направлении `step` (блокированные пропускаются). */
  function move(from: number, step: number): number {
    for (let n = 1; n <= options.length; n++) {
      const i = (from + step * n + options.length * n) % options.length;
      if (!isBlocked(i)) return i;
    }
    return from;
  }

  function onKeydown(event: KeyboardEvent): void {
    const { key } = event;
    if (key === 'ArrowDown' || key === 'ArrowUp') {
      event.preventDefault();
      if (!open) show();
      else active = move(active, key === 'ArrowDown' ? 1 : -1);
    } else if (key === 'Home' || key === 'End') {
      if (!open) return;
      event.preventDefault();
      active = key === 'Home' ? move(-1, 1) : move(options.length, -1);
    } else if (key === 'Enter' || key === ' ') {
      event.preventDefault();
      if (open) choose(active);
      else show();
    } else if (key === 'Escape' && open) {
      // Диалог под списком остаётся открытым: Esc закрывает только список.
      event.stopPropagation();
      close();
    } else if (key === 'Tab') {
      close();
    } else if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      // Поиск набором: «ка» переходит к первому пункту, который начинается с «ка».
      clearTimeout(typedTimer);
      typed = (typed + key).toLowerCase();
      typedTimer = setTimeout(() => {
        typed = '';
      }, 700);
      const found = options.findIndex(
        (o, i) => !isBlocked(i) && o.label.toLowerCase().startsWith(typed)
      );
      if (found >= 0) {
        if (!open) show();
        active = found;
      }
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
  <span class="lbl" id="{id}-label" class:sr-only={hideLabel}>{label}</span>
  <div class="box" bind:this={box}>
    <button
      {id}
      type="button"
      class="trigger"
      role="combobox"
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-controls={listId}
      aria-labelledby="{id}-label"
      aria-activedescendant={open ? optionId(active) : undefined}
      data-escape-closes-list={open}
      {disabled}
      onclick={() => {
        if (open) close();
        else show();
      }}
      onblur={close}
      onkeydown={onKeydown}
    >
      <span class="value">{selectedLabel}</span>
      <ChevronDown class="chevron" size={16} aria-hidden="true" />
    </button>
    {#if open}
      <ul id={listId} role="listbox" aria-labelledby="{id}-label" style={listStyle}>
        {#each options as option, i (option.value)}
          {@const reason = blocked?.(option.value) ?? null}
          <li
            id={optionId(i)}
            role="option"
            aria-selected={option.value === value}
            aria-disabled={reason !== null}
            title={reason ?? undefined}
            class:active={i === active}
            class:blocked={reason !== null}
            onpointermove={() => {
              if (reason === null) active = i;
            }}
            onpointerdown={(event) => {
              // Кнопка не должна терять фокус раньше выбора.
              event.preventDefault();
              choose(i);
            }}
          >
            <span class="text">{option.label}</span>
            {#if option.value === value}<Check size={14} aria-hidden="true" />{/if}
          </li>
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
  .lbl {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .box {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    width: 100%;
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fs-14) var(--font-sans);
    text-align: start;
    cursor: pointer;
  }
  .value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trigger :global(.chevron) {
    flex: none;
    color: var(--muted);
  }
  .trigger:focus-visible {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  .trigger:disabled {
    opacity: 0.45;
    cursor: default;
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
    justify-content: space-between;
    gap: var(--sp-3);
    min-height: var(--h-control-sm);
    padding-inline: var(--sp-3);
    border-radius: var(--r-xs);
    font-size: var(--fs-14);
    cursor: pointer;
  }
  li.active {
    background: var(--accent-soft);
  }
  li.blocked {
    color: var(--faint);
    cursor: not-allowed;
  }
</style>

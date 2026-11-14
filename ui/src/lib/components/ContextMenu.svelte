<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { MenuItem } from './menu';
  import { placeMenu } from './popover';

  type Props = {
    items: readonly MenuItem[];
    /** Точка вызова в координатах окна (clientX, clientY). */
    x: number;
    y: number;
    label: string;
    onselect: (id: string) => void;
    onclose: () => void;
  };

  let { items, x, y, label, onselect, onclose }: Props = $props();

  let menu = $state<HTMLUListElement>();
  let left = $state(0);
  let top = $state(0);
  let placed = $state(false);

  // Фокус, с которого открыли меню: после закрытия он возвращается туда, если не ушёл в другое окно.
  const opener = document.activeElement;
  onDestroy(() => {
    setTimeout(() => {
      if (
        document.activeElement === document.body &&
        opener instanceof HTMLElement &&
        opener.isConnected
      ) {
        opener.focus();
      }
    }, 0);
  });

  // Эффект идёт после вставки в DOM и до отрисовки: меню измеряется, сдвигается внутрь окна и
  // получает фокус на первом пункте. Размеры считаются в тех же единицах, что и в Tooltip
  // (экранные), а в CSS переводятся делением на масштаб интерфейса.
  $effect(() => {
    const point = { x, y };
    if (!menu) return;
    const zoom = parseFloat(document.documentElement.style.zoom) || 1;
    const box = menu.getBoundingClientRect();
    const spot = placeMenu({
      x: point.x,
      y: point.y,
      width: box.width,
      height: box.height,
      viewportWidth: document.documentElement.clientWidth,
      viewportHeight: document.documentElement.clientHeight
    });
    left = spot.left / zoom;
    top = spot.top / zoom;
    placed = true;
    menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
  });

  /** Нажатие вне меню закрывает его. */
  function outside(event: Event): void {
    if (!(event.target instanceof Node && menu?.contains(event.target))) onclose();
  }

  function choose(item: MenuItem): void {
    if (item.disabled) return;
    onselect(item.id);
    onclose();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onclose();
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      onclose();
      return;
    }
    const buttons = [...(menu?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])];
    if (buttons.length === 0) return;
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const step: Record<string, number | undefined> = {
      ArrowDown: at + 1,
      ArrowUp: at - 1,
      Home: 0,
      End: buttons.length - 1
    };
    const target = step[event.key];
    if (target === undefined) return;
    event.preventDefault();
    buttons[(target + buttons.length) % buttons.length]?.focus();
  }
</script>

<svelte:window onresize={onclose} onblur={onclose} onpointerdown={outside} onwheel={onclose} />

<ul
  bind:this={menu}
  class="menu"
  class:placed
  role="menu"
  aria-label={label}
  tabindex="-1"
  style:left="{left}px"
  style:top="{top}px"
  onkeydown={onKeydown}
  oncontextmenu={(event) => {
    event.preventDefault();
  }}
>
  {#each items as item (item.id)}
    {#if item.separator}<li class="sep" role="separator"></li>{/if}
    <li role="none">
      <button
        type="button"
        class="item"
        class:danger={item.danger}
        role={item.checked === undefined ? 'menuitem' : 'menuitemradio'}
        aria-checked={item.checked}
        disabled={item.disabled}
        tabindex="-1"
        onclick={() => {
          choose(item);
        }}
      >
        <span class="mark" aria-hidden="true">{item.checked ? '✓' : ''}</span>
        <span class="text">{item.label}</span>
        {#if item.hint}<span class="hint">{item.hint}</span>{/if}
      </button>
    </li>
  {/each}
</ul>

<style>
  .menu {
    position: fixed;
    z-index: var(--z-palette);
    min-width: 200px;
    max-width: 320px;
    margin: 0;
    padding: var(--sp-1);
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    box-shadow: var(--shadow);
    font-size: var(--fs-13);
    /* До измерения меню невидимо, чтобы не мигать в неверной точке. */
    opacity: 0;
  }
  .menu.placed {
    opacity: 1;
    transition: opacity var(--dur-fast) var(--ease-out);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    width: 100%;
    min-height: 32px;
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item:hover:not(:disabled),
  .item:focus-visible {
    background: var(--line-2);
    outline: none;
  }
  .item:disabled {
    color: var(--muted);
    cursor: default;
  }
  .item.danger {
    color: var(--danger);
  }
  .mark {
    flex-shrink: 0;
    width: 12px;
    color: var(--primary);
  }
  .text {
    flex-grow: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .sep {
    height: 1px;
    margin: var(--sp-1) 0;
    background: var(--line);
  }
</style>

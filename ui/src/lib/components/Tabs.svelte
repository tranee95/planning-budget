<script lang="ts" generics="T extends string">
  import { indicator } from '$lib/motion/indicator';

  type Props = {
    tabs: readonly { value: T; label: string }[];
    value: T;
    label: string;
    onchange: (value: T) => void;
  };

  let { tabs, value, label, onchange }: Props = $props();

  function onKeydown(event: KeyboardEvent, index: number): void {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const next = tabs[(index + step + tabs.length) % tabs.length];
    if (!next) return;
    onchange(next.value);
    const list = (event.currentTarget as HTMLElement).parentElement;
    list?.querySelectorAll<HTMLElement>('[role=tab]')[tabs.indexOf(next)]?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={label} use:indicator>
  <span class="thumb" aria-hidden="true"></span>
  {#each tabs as tab, index (tab.value)}
    <button
      type="button"
      role="tab"
      aria-selected={value === tab.value}
      tabindex={value === tab.value ? 0 : -1}
      class:active={value === tab.value}
      onclick={() => {
        onchange(tab.value);
      }}
      onkeydown={(event) => {
        onKeydown(event, index);
      }}
    >
      {tab.label}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: var(--sp-5);
    position: relative;
    border-bottom: 1px solid var(--line);
  }
  /* Подчёркивание активной вкладки скользит (lib/motion/indicator): бокс по размеру кнопки, видна нижняя линия. */
  .thumb {
    position: absolute;
    top: 0;
    left: 0;
    border-bottom: 2px solid var(--accent);
    pointer-events: none;
    will-change: transform;
  }
  button {
    height: var(--h-control);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: var(--fw-medium) var(--fs-14) var(--font-sans);
    cursor: pointer;
    transition: color var(--dur-base) var(--ease-out);
  }
  button:hover {
    color: var(--ink);
  }
  button.active {
    color: var(--ink);
  }
  button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

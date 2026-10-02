<script lang="ts" generics="T extends string">
  import { indicator } from '$lib/motion/indicator';

  type Props = {
    options: readonly { value: T; label: string }[];
    value: T;
    label: string;
    onchange: (value: T) => void;
  };

  let { options, value, label, onchange }: Props = $props();

  function onKeydown(event: KeyboardEvent, index: number): void {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const next = options[(index + step + options.length) % options.length];
    if (!next) return;
    onchange(next.value);
    const group = (event.currentTarget as HTMLElement).parentElement;
    group?.querySelectorAll<HTMLElement>('[role=radio]')[options.indexOf(next)]?.focus();
  }
</script>

<div class="group" role="radiogroup" aria-label={label} use:indicator>
  <span class="thumb" aria-hidden="true"></span>
  {#each options as option, index (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === option.value}
      tabindex={value === option.value ? 0 : -1}
      class:active={value === option.value}
      onclick={() => {
        onchange(option.value);
      }}
      onkeydown={(event) => {
        onKeydown(event, index);
      }}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .group {
    display: inline-flex;
    /* Не помещается в узкую панель — переносится на второй ряд, а не выходит за край. */
    flex-wrap: wrap;
    max-width: 100%;
    gap: var(--sp-1);
    padding: var(--sp-1);
    border-radius: var(--r-md);
    background: var(--line-2);
  }
  .group {
    position: relative;
  }
  /* Подложка выбранного пункта скользит между кнопками (lib/motion/indicator). */
  .thumb {
    position: absolute;
    top: 0;
    left: 0;
    border-radius: var(--r-sm);
    background: var(--surface);
    box-shadow: var(--shadow);
    pointer-events: none;
    will-change: transform;
  }
  button {
    position: relative;
    z-index: 1;
    height: var(--h-control-sm);
    padding-inline: var(--sp-3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink-2);
    font: var(--fw-medium) var(--fs-13) var(--font-sans);
    line-height: var(--h-control-sm);
    white-space: nowrap;
    cursor: pointer;
    transition: color var(--dur-base) var(--ease-out);
  }
  button.active {
    color: var(--ink);
  }
  button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

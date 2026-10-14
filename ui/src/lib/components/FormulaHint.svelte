<script lang="ts">
  import CircleHelp from '@lucide/svelte/icons/circle-help';
  import { clampShift } from './popover';

  type Props = {
    /** Что объясняется: «Свободный остаток». Попадает в имя кнопки для скринридера. */
    label: string;
    /** Строки формулы: первая — сама формула, дальше — что в неё входит и крайние случаи. */
    lines: readonly string[];
  };

  let { label, lines }: Props = $props();

  const id = $props.id();
  let wrap = $state<HTMLSpanElement>();
  let tip = $state<HTMLSpanElement>();
  let shift = $state(0);
  let open = $state(false);
  let placed = $state(false);

  // Закрытая подсказка не занимает места (display: none), поэтому у края окна не даёт прокрутку.
  // Открытая сдвигается внутрь окна, а не обрезается; до расчёта сдвига она скрыта.
  $effect(() => {
    if (!open) {
      placed = false;
      return;
    }
    if (wrap && tip) {
      const zoom = parseFloat(document.documentElement.style.zoom) || 1;
      const w = wrap.getBoundingClientRect();
      shift =
        clampShift({
          center: w.left + w.width / 2,
          width: tip.getBoundingClientRect().width,
          viewport: document.documentElement.clientWidth
        }) / zoom;
    }
    placed = true;
    // Escape закрывает подсказку, не сдвигая фокус (WCAG 1.4.13).
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') open = false;
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
    };
  });

  // Клик мышью оставляет фокус на кнопке, но подсказку открывает только фокус с клавиатуры.
  function onFocusIn(event: FocusEvent): void {
    if ((event.target as Element).matches(':focus-visible')) open = true;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  class="wrap"
  bind:this={wrap}
  onpointerenter={() => (open = true)}
  onpointerleave={() => (open = false)}
  onfocusin={onFocusIn}
  onfocusout={() => (open = false)}
>
  <button class="btn" type="button" aria-label="Как считается: {label}" aria-describedby={id}>
    <CircleHelp size={14} aria-hidden="true" />
  </button>
  <span
    class="tip"
    class:open
    class:placed
    role="tooltip"
    {id}
    bind:this={tip}
    style:--shift="{shift}px"
  >
    <b>{label}</b>
    {#each lines as line, i (i)}<span class:formula={i === 0}>{line}</span>{/each}
  </span>
</span>

<style>
  .wrap {
    position: relative;
    display: inline-flex;
    flex: none;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    /* Значок 14 px, область клика 24 px (WCAG 2.5.8). Отрицательные поля сверху, снизу и слева
       сохраняют прежние зазоры; справа поля нет: иначе кнопка выходит за строку и обрезается. */
    /* При масштабе интерфейса меньше 100 % размер растёт, чтобы на экране оставалось 24 px. */
    --hint-size: calc(var(--sp-6) / var(--ui-zoom, 1));
    width: var(--hint-size);
    height: var(--hint-size);
    padding: 0;
    margin-block: calc((14px - var(--hint-size)) / 2);
    margin-inline-start: calc((14px - var(--hint-size)) / 2);
    border: 0;
    border-radius: var(--r-full);
    background: transparent;
    color: var(--muted);
    cursor: help;
    transition: color var(--dur-base) var(--ease-out);
  }
  .btn:hover,
  .btn:focus-visible {
    color: var(--ink);
  }
  .btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .tip {
    position: absolute;
    inset-block-start: calc(100% + var(--sp-1));
    inset-inline-start: 50%;
    z-index: var(--z-tooltip);
    display: none;
    flex-direction: column;
    gap: var(--sp-1);
    width: max-content;
    max-width: 300px;
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--r-xs);
    background: var(--ink);
    color: var(--bg);
    font-size: var(--fs-12);
    font-weight: var(--fw-regular);
    line-height: var(--lh-body);
    text-align: start;
    white-space: normal;
    visibility: hidden;
    opacity: 0;
    transform: translateX(calc(-50% + var(--shift, 0px)));
    transition: opacity var(--dur-fast) var(--ease-out);
  }
  .formula {
    font-variant-numeric: tabular-nums;
    font-weight: var(--fw-medium);
  }
  .tip.open {
    display: flex;
  }
  .tip.open.placed {
    visibility: visible;
    opacity: 1;
  }
  /* Мост через зазор: указатель переходит с кнопки на подсказку, не закрывая её. */
  .tip::before {
    content: '';
    position: absolute;
    inset-block-start: calc(-1 * var(--sp-1));
    inset-inline: 0;
    height: var(--sp-1);
  }
</style>

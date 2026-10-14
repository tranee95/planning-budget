<script lang="ts">
  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import { formatMoney } from '$lib/format';
  import { dur } from '$lib/motion';
  import type { FormulaText } from '$lib/formulas';
  import FormulaHint from './FormulaHint.svelte';

  type Props = {
    label: string;
    /** Уже отформатированное значение (`formatMoney`). Не используется, если задан `amount`. */
    value?: string;
    /** Сумма в копейках: число «докручивается» к новому значению при смене месяца. */
    amount?: number;
    /** Инсайт одной строкой: «−8% к среднему». */
    hint?: string;
    /** Как считается значение: рядом с подписью появляется «?» с формулой. */
    formula?: FormulaText;
    children?: Snippet;
  };

  let { label, value = '', amount, hint, formula, children }: Props = $props();

  const tween = new Tween(0, { easing: cubicOut });
  $effect(() => {
    if (amount === undefined) return;
    const target = amount;
    // Длительность читает состояние самой анимации: без untrack эффект перезапускался бы каждый кадр.
    const duration = untrack(() => (tween.target === 0 && tween.current === 0 ? 0 : dur.slow()));
    void tween.set(target, { duration });
  });
  const shown = $derived(amount === undefined ? value : formatMoney(Math.round(tween.current)));
</script>

<div class="kpi">
  <span class="label">
    {label}
    {#if formula}<FormulaHint label={formula.label ?? label} lines={formula.lines} />{/if}
  </span>
  <span class="value num">{shown}</span>
  {#if hint}<span class="hint">{hint}</span>{/if}
  {@render children?.()}
</div>

<style>
  .kpi {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    min-width: 0;
  }
  .label {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .value {
    font-size: var(--fs-26);
    font-weight: var(--fw-semibold);
    letter-spacing: var(--tracking-tight);
    line-height: var(--lh-tight);
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--muted);
    font-size: var(--fs-12);
  }
</style>

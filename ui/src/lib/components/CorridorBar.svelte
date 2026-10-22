<script lang="ts">
  import { formatPercentWhole } from '$lib/format';
  type Props = {
    /** Фактическая норма сбережений, базисные пункты. */
    value: number;
    min: number;
    max: number;
    /** Верх шкалы; по умолчанию с запасом над верхней границей коридора. */
    scaleMax?: number;
  };

  let { value, min, max, scaleMax = Math.round(max * 1.5) }: Props = $props();

  const pct = (bp: number): number => Math.min(Math.max(bp / scaleMax, 0), 1) * 100;
  const state = $derived(value < min ? 'below' : value > max ? 'above' : 'inside');
  const text = $derived(
    state === 'below' ? 'ниже коридора' : state === 'above' ? 'выше коридора' : 'в коридоре'
  );
</script>

<div class="wrap">
  <div
    class="corridor"
    role="img"
    aria-label="Сбережения {(value / 100).toLocaleString('ru-RU')} %, {text}"
  >
    <span
      class="zone"
      style:inset-inline-start="{pct(min)}%"
      style:inline-size="{pct(max) - pct(min)}%"
    ></span>
    <span class="marker {state}" style:inset-inline-start="{pct(value)}%"></span>
  </div>
  <!-- границы коридора подписаны: зелёная зона без цифр непонятна -->
  <div class="scale" aria-hidden="true">
    <span style:inset-inline-start="{pct(min)}%">{formatPercentWhole(min)}</span>
    <span style:inset-inline-start="{pct(max)}%">{formatPercentWhole(max)}</span>
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }
  .scale {
    position: relative;
    height: 14px;
    color: var(--muted);
    font-size: var(--fs-11);
  }
  /* подписи границ расходятся от зоны: нижняя уходит влево, верхняя — вправо, не накладываясь */
  .scale span {
    position: absolute;
    white-space: nowrap;
  }
  .scale span:first-child {
    transform: translateX(-100%);
  }
  .corridor {
    position: relative;
    height: 6px;
    border-radius: var(--r-full);
    background: var(--line-2);
  }
  .zone {
    position: absolute;
    inset-block: 0;
    background: var(--paid-bar);
    opacity: 0.35;
  }
  .marker {
    position: absolute;
    inset-block: -3px;
    width: 3px;
    margin-inline-start: -1px;
    border-radius: var(--r-full);
    background: var(--ink);
  }
  .below {
    background: var(--debt-bar);
  }
  .above {
    background: var(--paid-bar);
  }
</style>

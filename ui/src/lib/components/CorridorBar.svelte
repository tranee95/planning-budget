<script lang="ts">
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

<style>
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

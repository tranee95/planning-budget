<script lang="ts">
  type Props = {
    /** Оплачено, доля лимита; 1 = лимит исчерпан. Может быть больше 1. */
    value: number;
    /** Запланировано сверх оплаченного, та же шкала. */
    planned?: number;
    label: string;
    /** `limit` — расход против лимита (жёлтый с 90 %, красный от 100 %); `goal` — движение к цели (долг, накопление): 100 % — хорошо. */
    kind?: 'limit' | 'goal';
  };

  let { value, planned = 0, label, kind = 'limit' }: Props = $props();

  const level = $derived(
    kind === 'goal' ? 'ok' : value + planned >= 1 ? 'over' : value + planned >= 0.9 ? 'warn' : 'ok'
  );
  const paidScale = $derived(Math.min(Math.max(value, 0), 1));
  const plannedScale = $derived(Math.min(Math.max(value + planned, 0), 1));
</script>

<div
  class="progress {level}"
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round((value + planned) * 100)}
>
  <span class="bar plan" style:transform="scaleX({plannedScale})"></span>
  <span class="bar paid" style:transform="scaleX({paidScale})"></span>
</div>

<style>
  .progress {
    position: relative;
    height: 6px;
    overflow: hidden;
    border-radius: var(--r-full);
    background: var(--line-2);
  }
  .bar {
    position: absolute;
    inset: 0;
    transform-origin: left;
    /* ширина — scaleX за 320 мс, смена цвета на 90 % и 100 % плавная */
    transition:
      transform var(--dur-slow) var(--ease-out),
      background-color var(--dur-slow) var(--ease-out);
    animation: grow var(--dur-progress) var(--ease-out);
  }
  .paid {
    background: var(--paid-bar);
  }
  .warn .paid {
    background: var(--debt-bar);
  }
  .over .paid {
    background: var(--unpl-bar);
  }
  /* лимит исчерпан: один короткий пульс полосы */
  .over {
    animation: limit-pulse var(--dur-slow) var(--ease-out);
  }
  @keyframes limit-pulse {
    40% {
      transform: scaleY(1.9);
    }
  }
  .plan {
    background: repeating-linear-gradient(135deg, var(--plan-bar) 0 4px, transparent 4px 8px);
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
</style>

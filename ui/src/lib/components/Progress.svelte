<script lang="ts">
  type Props = {
    /** Оплачено, доля лимита; 1 = лимит исчерпан. Может быть больше 1. */
    value: number;
    /** Запланировано сверх оплаченного, та же шкала. */
    planned?: number;
    label: string;
  };

  let { value, planned = 0, label }: Props = $props();

  const level = $derived(value + planned > 1 ? 'over' : value + planned >= 0.85 ? 'warn' : 'ok');
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
    transition: transform var(--dur-base) var(--ease-out);
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
  .plan {
    background: repeating-linear-gradient(135deg, var(--plan-bar) 0 4px, transparent 4px 8px);
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
</style>

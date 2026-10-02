<script lang="ts">
  import type { TxStatusDto } from '$lib/api/bindings';
  import { STATUS_LABEL } from './status';

  let { status, size = 14 }: { status: TxStatusDto; size?: number } = $props();
</script>

<!-- Статус передаётся формой, а не только цветом: круг, ромб, треугольник, кольцо. -->
<svg
  class="dot {status}"
  width={size}
  height={size}
  viewBox="0 0 14 14"
  role="img"
  aria-label={STATUS_LABEL[status]}
>
  {#if status === 'paid'}
    <circle cx="7" cy="7" r="5.5" />
  {:else if status === 'debt'}
    <path d="M7 1 13 7 7 13 1 7Z" />
  {:else if status === 'unplanned'}
    <path d="M7 1.5 13 12.5H1Z" />
  {:else}
    <circle cx="7" cy="7" r="5" fill="none" stroke-width="1.75" />
  {/if}
</svg>

<style>
  .dot {
    flex-shrink: 0;
  }
  .paid {
    fill: var(--paid-bar);
  }
  .debt {
    fill: var(--debt-bar);
  }
  .unplanned {
    fill: var(--unpl-bar);
  }
  .planned {
    stroke: var(--plan);
  }
</style>

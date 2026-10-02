<script lang="ts">
  import type { StatusAmountsDto, TxStatusDto } from '$lib/api/bindings';
  import { StatusChip } from '$lib/components';
  import { STATUS_ORDER } from '$lib/components/status';
  import { formatMoney } from '$lib/format';

  let { byStatus }: { byStatus: StatusAmountsDto } = $props();

  const total = $derived(STATUS_ORDER.reduce((sum, s) => sum + byStatus[s], 0));
  const share = (s: TxStatusDto): number => (total === 0 ? 0 : (byStatus[s] / total) * 100);
</script>

<div class="strip">
  <span class="lbl">Статусы трат</span>
  <div class="track" role="img" aria-label="Доли расходов по статусам">
    {#each STATUS_ORDER as status (status)}
      {#if byStatus[status] > 0}
        <span class="seg {status}" style:width="{share(status)}%"></span>
      {/if}
    {/each}
  </div>
  <div class="chips">
    {#each STATUS_ORDER as status (status)}
      <span class="item"
        ><StatusChip {status} /><b class="num">{formatMoney(byStatus[status])}</b></span
      >
    {/each}
  </div>
</div>

<style>
  .strip {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sp-4) var(--sp-6);
    padding: var(--sp-4) var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .lbl {
    min-width: 96px;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .track {
    display: flex;
    flex: 1 1 200px;
    height: 10px;
    gap: 2px;
    overflow: hidden;
    border-radius: var(--r-full);
    background: var(--line-2);
  }
  .seg {
    height: 100%;
    transform-origin: left;
    animation: grow var(--dur-progress) var(--ease-out) both;
  }
  .paid {
    background: var(--paid-bar);
  }
  .debt {
    background: var(--debt-bar);
  }
  .unplanned {
    background: var(--unpl-bar);
  }
  .planned {
    background: var(--plan-bar);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2) var(--sp-4);
  }
  .item {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-13);
  }
  .item b {
    font-weight: var(--fw-semibold);
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
</style>

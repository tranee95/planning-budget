<script lang="ts">
  import type { TxStatusDto } from '$lib/api/bindings';
  import { STATUS_LABEL } from './status';

  let { status }: { status: TxStatusDto } = $props();
</script>

<span class="chip {status}">{STATUS_LABEL[status]}</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding-inline: var(--sp-2);
    border-radius: var(--r-full);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
    white-space: nowrap;
  }
  .paid {
    background: var(--paid-bg);
    color: var(--paid);
  }
  .debt {
    background: var(--debt-bg);
    color: var(--debt);
  }
  .unplanned {
    background: var(--unpl-bg);
    color: var(--unpl);
  }
  .planned {
    border: 1px dashed var(--plan);
    color: var(--plan);
  }
</style>

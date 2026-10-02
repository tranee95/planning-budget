<script lang="ts">
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { formatMonth } from '$lib/format';
  import { month } from '$lib/stores/month.svelte';
  import IconButton from './IconButton.svelte';
</script>

<div class="switcher">
  <IconButton
    size="sm"
    aria-label="Предыдущий месяц"
    aria-keyshortcuts="ArrowLeft"
    onclick={() => {
      month.shift(-1);
    }}
  >
    <ChevronLeft size={18} aria-hidden="true" />
  </IconButton>
  <span class="label" aria-live="polite">{formatMonth(month.current)}</span>
  <IconButton
    size="sm"
    aria-label="Следующий месяц"
    aria-keyshortcuts="ArrowRight"
    onclick={() => {
      month.shift(1);
    }}
  >
    <ChevronRight size={18} aria-hidden="true" />
  </IconButton>
</div>

<style>
  .switcher {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
  }
  .label {
    min-width: 128px;
    text-align: center;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
</style>

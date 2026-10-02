<script lang="ts">
  import type { TxStatusDto } from '$lib/api/bindings';
  import StatusDot from './StatusDot.svelte';
  import { STATUS_LABEL, STATUS_ORDER } from './status';

  type Props = { value: TxStatusDto; onchange: (value: TxStatusDto) => void };

  let { value, onchange }: Props = $props();
</script>

<div class="group" role="radiogroup" aria-label="Статус">
  {#each STATUS_ORDER as status (status)}
    <button
      type="button"
      role="radio"
      aria-checked={value === status}
      class:active={value === status}
      onclick={() => {
        onchange(status);
      }}
    >
      <StatusDot {status} size={12} />{STATUS_LABEL[status]}
    </button>
  {/each}
</div>

<style>
  .group {
    display: inline-flex;
    flex-wrap: wrap;
    max-width: 100%;
    gap: var(--sp-1);
    padding: var(--sp-1);
    border-radius: var(--r-md);
    background: var(--line-2);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    height: var(--h-control-sm);
    padding-inline: var(--sp-3);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink-2);
    font: var(--fw-medium) var(--fs-13) var(--font-sans);
    cursor: pointer;
    transition: background-color var(--dur-base) var(--ease-out);
  }
  button.active {
    background: var(--surface);
    box-shadow: var(--shadow);
    color: var(--ink);
  }
  button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>

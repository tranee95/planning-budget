<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { FormulaText } from '$lib/formulas';
  import FormulaHint from './FormulaHint.svelte';

  type Props = { title?: string; formula?: FormulaText; actions?: Snippet; children: Snippet };

  let { title, formula, actions, children }: Props = $props();
</script>

<section class="card">
  {#if title || actions}
    <header>
      {#if title}
        <div class="heading">
          <h2>{title}</h2>
          {#if formula}<FormulaHint label={formula.label ?? title} lines={formula.lines} />{/if}
        </div>
      {/if}
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </header>
  {/if}
  {@render children()}
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
  }
  .heading {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
  }
</style>

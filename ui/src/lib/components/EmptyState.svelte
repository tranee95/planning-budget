<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = { title: string; hint?: string; actions?: Snippet };

  let { title, hint, actions }: Props = $props();
</script>

<div class="empty">
  <p class="title">{title}</p>
  {#if hint}<p class="hint">{hint}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-10) var(--sp-6);
    text-align: center;
  }
  p {
    margin: 0;
  }
  .title {
    font-size: var(--fs-15);
    font-weight: var(--fw-medium);
  }
  .hint {
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
    margin-top: var(--sp-3);
  }
</style>

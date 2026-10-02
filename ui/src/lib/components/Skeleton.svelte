<script lang="ts">
  type Props = { width?: string; height?: string };

  let { width = '100%', height = '16px' }: Props = $props();
</script>

<span class="skeleton" style:width style:height aria-hidden="true"></span>

<style>
  .skeleton {
    display: block;
    border-radius: var(--r-xs);
    background: var(--line-2);
    animation: pulse 1.2s var(--ease-in-out) infinite alternate;
  }
  @keyframes pulse {
    from {
      opacity: 1;
    }
    to {
      opacity: 0.5;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton {
      animation: none;
    }
  }
  :global(:root[data-motion='reduce']) .skeleton {
    animation: none;
  }
</style>

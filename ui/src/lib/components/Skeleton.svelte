<script lang="ts">
  type Props = { width?: string; height?: string };

  let { width = '100%', height = '16px' }: Props = $props();
</script>

<span class="skeleton" style:width style:height aria-hidden="true"></span>

<style>
  /* Показывается, только если загрузка длится дольше 150 мс: быстрые ответы не мигают заглушкой */
  .skeleton {
    position: relative;
    display: block;
    overflow: hidden;
    border-radius: var(--r-xs);
    background: var(--line-2);
    opacity: 0;
    animation: reveal 0s linear 150ms forwards;
  }
  /* блик 1,2 с: двигается только transform */
  .skeleton::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--surface-2), transparent);
    transform: translateX(-100%);
    animation: shimmer 1.2s var(--ease-in-out) infinite;
  }
  @keyframes reveal {
    to {
      opacity: 1;
    }
  }
  @keyframes shimmer {
    to {
      transform: translateX(100%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton::after {
      animation: none;
    }
  }
  :global(:root[data-motion='reduce']) .skeleton::after {
    animation: none;
  }
</style>

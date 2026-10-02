<script lang="ts">
  type Props = { size?: number };

  let { size = 56 }: Props = $props();
</script>

<span class="logo" style:--size="{size}px" aria-hidden="true">
  <i class="b1"></i><i class="b2"></i><i class="b3"></i>
</span>

<style>
  .logo {
    display: inline-flex;
    align-items: flex-end;
    justify-content: center;
    gap: calc(var(--size) * 0.07);
    box-sizing: border-box;
    width: var(--size);
    height: var(--size);
    padding-bottom: calc(var(--size) * 0.23);
    border-radius: calc(var(--size) * 0.3);
    background: var(--ink);
  }
  i {
    width: calc(var(--size) * 0.125);
    border-radius: 2px;
    background: var(--bg);
  }
  .b1 {
    height: calc(var(--size) * 0.21);
  }
  .b2 {
    height: calc(var(--size) * 0.43);
  }
  .b3 {
    height: calc(var(--size) * 0.3);
    background: var(--accent);
  }
</style>

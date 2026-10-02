<script lang="ts">
  import { passwordStrength } from '$lib/security/password';

  type Props = { password: string };

  let { password }: Props = $props();

  const strength = $derived(passwordStrength(password));
  const filled = $derived(password === '' ? 0 : strength.score + 1);
</script>

{#if password !== ''}
  <div class="meter" role="status">
    <div class="bars" aria-hidden="true">
      {#each [1, 2, 3, 4] as n (n)}
        <span class:on={n <= filled} class="bar s{strength.score}"></span>
      {/each}
    </div>
    <span class="label">Надёжность: {strength.label.toLowerCase()}</span>
  </div>
  {#if strength.popular}
    <p class="warn">Этот пароль есть в списке самых распространённых. Лучше выбрать другой.</p>
  {/if}
{/if}

<style>
  .meter {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    width: 100%;
    margin-top: var(--sp-3);
  }
  .bars {
    display: flex;
    flex: 1;
    gap: var(--sp-1);
  }
  .bar {
    flex: 1;
    height: 4px;
    border-radius: var(--r-full);
    background: var(--line-2);
  }
  .on.s0 {
    background: var(--unpl-bar);
  }
  .on.s1 {
    background: var(--debt-bar);
  }
  .on.s2,
  .on.s3 {
    background: var(--paid-bar);
  }
  .label {
    color: var(--muted);
    font-size: var(--fs-12);
    white-space: nowrap;
  }
  .warn {
    width: 100%;
    margin: var(--sp-2) 0 0;
    color: var(--debt);
    font-size: var(--fs-12);
  }
</style>

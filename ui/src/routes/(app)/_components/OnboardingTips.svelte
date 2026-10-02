<script lang="ts">
  import { Button, Kbd } from '$lib/components';
  import { toastSlide } from '$lib/motion';
  import { TIPS } from '$lib/tips';

  type Props = {
    /** Пройдены все подсказки или пользователь отключил их. */
    ondone: () => void;
  };

  let { ondone }: Props = $props();

  let step = $state(0);
  const tip = $derived(TIPS[step]);
  const last = $derived(step === TIPS.length - 1);
</script>

{#if tip}
  <section class="tips" aria-label="Подсказки для начала работы" transition:toastSlide>
    <p class="step">{step + 1} из {TIPS.length}</p>
    <h2>{tip.title}</h2>
    <p class="text">
      {tip.text}
      {#if tip.keys}
        <span class="keys">
          {#each tip.keys as key (key)}<Kbd>{key}</Kbd>{/each}
        </span>
      {/if}
    </p>
    <div class="actions">
      <Button variant="ghost" size="sm" onclick={ondone}>Больше не показывать</Button>
      <Button
        variant="primary"
        size="sm"
        onclick={() => {
          if (last) ondone();
          else step += 1;
        }}>{last ? 'Понятно' : 'Далее'}</Button
      >
    </div>
  </section>
{/if}

<style>
  /* Обычный блок над экраном, а не плавающая карточка: ничего не перекрывает. */
  .tips {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    max-width: 640px;
    margin-bottom: var(--sp-4);
    padding: var(--sp-4) var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .step {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
  .text {
    margin: 0;
    color: var(--ink-2);
    font-size: var(--fs-13);
    line-height: var(--lh-body);
  }
  .keys {
    display: inline-flex;
    gap: var(--sp-1);
    margin-left: var(--sp-1);
    vertical-align: middle;
  }
  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    margin-top: var(--sp-2);
  }
</style>

<script lang="ts">
  import { Button, Modal, StatusDot } from '$lib/components';
  import { INTRO_STEPS, STATUS_EXPLAINED } from '$lib/help/content';

  type Props = {
    /** Знакомство пройдено, пропущено или закрыто: больше не показывать. */
    onfinish: () => void;
    /** Последний шаг: сразу перейти к мастеру первого месяца. */
    onplan: () => void;
  };

  let { onfinish, onplan }: Props = $props();

  let step = $state(0);
  const current = $derived(INTRO_STEPS[step]);
  const last = $derived(step === INTRO_STEPS.length - 1);

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowRight' && !last) {
      event.preventDefault();
      step += 1;
    } else if (event.key === 'ArrowLeft' && step > 0) {
      event.preventDefault();
      step -= 1;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if current}
  <Modal title={current.title} width={640} onclose={onfinish}>
    <p class="step" aria-live="polite">Шаг {step + 1} из {INTRO_STEPS.length}</p>
    {#each current.paragraphs as paragraph (paragraph)}
      <p class="text">{paragraph}</p>
    {/each}

    {#if current.id === 'cycle'}
      <ol class="flow" aria-label="Цикл месяца">
        {#each ['Доход', 'План', 'План готов', 'Оплата', 'Итоги'] as name, index (name)}
          <li class:key={name === 'План готов'}>{name}{index < 4 ? ' →' : ''}</li>
        {/each}
      </ol>
    {:else if current.id === 'statuses'}
      <ul class="statuses">
        {#each STATUS_EXPLAINED as item (item.status)}
          <li>
            <StatusDot status={item.status} />
            <b>{item.label}</b>
            <span>{item.text}</span>
          </li>
        {/each}
      </ul>
    {/if}

    <p class="dots" aria-hidden="true">
      {#each INTRO_STEPS as dot, index (dot.id)}
        <span class="dot" class:on={index === step}></span>
      {/each}
    </p>

    {#snippet footer()}
      <Button variant="ghost" onclick={onfinish}>{last ? 'Позже' : 'Пропустить'}</Button>
      {#if step > 0}
        <Button
          variant="secondary"
          onclick={() => {
            step -= 1;
          }}>Назад</Button
        >
      {/if}
      {#if last}
        <Button variant="primary" onclick={onplan}>Спланировать первый месяц</Button>
      {:else}
        <Button
          variant="primary"
          onclick={() => {
            step += 1;
          }}>Далее</Button
        >
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .step {
    margin: 0 0 var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .text {
    margin: 0 0 var(--sp-3);
    color: var(--ink-2);
    font-size: var(--fs-14);
    line-height: var(--lh-body);
  }
  .flow {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    margin: var(--sp-4) 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-13);
    color: var(--muted);
  }
  .flow li.key {
    color: var(--accent);
    font-weight: var(--fw-semibold);
  }
  .statuses {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    margin: var(--sp-3) 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-13);
  }
  .statuses li {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }
  .statuses span {
    color: var(--muted);
  }
  .dots {
    display: flex;
    justify-content: center;
    gap: var(--sp-2);
    margin: var(--sp-4) 0 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--line);
  }
  .dot.on {
    width: 24px;
    background: var(--accent);
  }
</style>

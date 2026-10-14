<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, Checkbox, Modal, StatusDot } from '$lib/components';
  import { INTRO_CATEGORIES_STEP, INTRO_STEPS, STATUS_EXPLAINED } from '$lib/help/content';
  import { CategoryPickerVm } from './category-picker.svelte';

  type Props = {
    /** Знакомство пройдено, пропущено или закрыто: больше не показывать. */
    onfinish: () => void;
    /** Последний шаг: сразу перейти к мастеру первого месяца. */
    onplan: () => void;
  };

  let { onfinish, onplan }: Props = $props();

  const vm = new CategoryPickerVm();
  onMount(() => {
    void vm.load();
  });

  let step = $state(0);
  // Выбор категорий идёт перед последним шагом и только пока категорий нет.
  const steps = $derived(
    vm.needed
      ? [...INTRO_STEPS.slice(0, -1), INTRO_CATEGORIES_STEP, ...INTRO_STEPS.slice(-1)]
      : INTRO_STEPS
  );
  const current = $derived(steps[step]);
  const last = $derived(step === steps.length - 1);

  /** Выход из знакомства создаёт отмеченные категории; при ошибке окно остаётся открытым. */
  async function leave(next: () => void): Promise<void> {
    // Escape или клик по фону во время сохранения не должны запускать выход дважды.
    if (vm.busy) return;
    if (await vm.apply()) next();
  }

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

{#if vm.ready && current}
  <Modal
    title={current.title}
    width={640}
    onclose={() => {
      // Escape и клик по фону не создают категории вслепую: выход идёт кнопками «Пропустить»
      // и «Позже», когда выбор ещё предстоит.
      if (!vm.needed) void leave(onfinish);
    }}
  >
    <p class="step" aria-live="polite">Шаг {step + 1} из {steps.length}</p>
    {#each current.paragraphs as paragraph (paragraph)}
      <p class="text">{paragraph}</p>
    {/each}

    {#if current.id === 'cycle'}
      <ol class="flow" aria-label="Цикл месяца">
        {#each ['Доход', 'План', 'План готов', 'Оплата', 'Итоги'] as name, index (name)}
          <li class:key={name === 'План готов'}>{name}{index < 4 ? ' →' : ''}</li>
        {/each}
      </ol>
    {:else if current.id === 'categories'}
      <div class="picker-actions">
        <Button
          variant="ghost"
          size="sm"
          onclick={() => {
            vm.setAll(true);
          }}>Отметить все</Button
        >
        <Button
          variant="ghost"
          size="sm"
          onclick={() => {
            vm.setAll(false);
          }}>Только обязательные</Button
        >
      </div>
      {#each vm.groups as group (group.kind)}
        <fieldset class="group">
          <legend>{group.label}</legend>
          <div class="items">
            {#each group.items as item (item.name)}
              <Checkbox
                disabled={item.required}
                bind:checked={
                  () => vm.isChosen(item.name),
                  () => {
                    vm.toggle(item.name);
                  }
                }
              >
                {item.name}{#if item.required}<span class="always"> · всегда</span>{/if}
              </Checkbox>
            {/each}
          </div>
        </fieldset>
      {/each}
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

    {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}

    <p class="dots" aria-hidden="true">
      {#each steps as dot, index (dot.id)}
        <span class="dot" class:on={index === step}></span>
      {/each}
    </p>

    {#snippet footer()}
      <Button
        variant="ghost"
        disabled={vm.busy}
        onclick={() => {
          void leave(onfinish);
        }}>{last ? 'Позже' : 'Пропустить'}</Button
      >
      {#if step > 0}
        <Button
          variant="secondary"
          onclick={() => {
            step -= 1;
          }}>Назад</Button
        >
      {/if}
      {#if last}
        <Button
          variant="primary"
          loading={vm.busy}
          onclick={() => {
            void leave(onplan);
          }}>Спланировать первый месяц</Button
        >
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
  .picker-actions {
    display: flex;
    gap: var(--sp-2);
    margin: 0 0 var(--sp-2);
  }
  .group {
    margin: 0 0 var(--sp-3);
    padding: 0;
    border: 0;
  }
  legend {
    padding: 0;
    margin: 0 0 var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .items {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0 var(--sp-5);
  }
  .always {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .error {
    margin: var(--sp-3) 0 0;
    color: var(--unpl);
    font-size: var(--fs-13);
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
    border-radius: var(--r-full);
    background: var(--line);
  }
  .dot.on {
    width: 24px;
    background: var(--accent);
  }
</style>

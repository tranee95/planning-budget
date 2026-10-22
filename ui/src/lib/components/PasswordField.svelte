<script lang="ts">
  import { shake as shakeNode } from '$lib/motion/shake';
  import Eye from '@lucide/svelte/icons/eye';
  import EyeOff from '@lucide/svelte/icons/eye-off';

  type Props = {
    id: string;
    label: string;
    value?: string;
    autocomplete: 'current-password' | 'new-password';
    error?: string | null;
    hint?: string | null;
    /** Растёт на 1 при каждой неудаче: поле дёргается по X. */
    shake?: number;
    autofocus?: boolean;
    disabled?: boolean;
    /** Сколько строк сообщения (ошибка, подсказка) держать свободными: форма не прыгает, когда сообщение появляется. */
    reserveLines?: number;
    /** `md` — формы внутри приложения; `lg` — экраны входа. */
    size?: 'md' | 'lg';
  };

  let {
    id,
    label,
    value = $bindable(''),
    autocomplete,
    error = null,
    hint = null,
    shake = 0,
    autofocus = false,
    disabled = false,
    reserveLines = 0,
    size = 'md'
  }: Props = $props();

  let visible = $state(false);
  let input = $state<HTMLInputElement>();
  let box = $state<HTMLDivElement>();

  $effect(() => {
    if (autofocus) input?.focus();
  });

  $effect(() => {
    if (shake === 0 || !box) return;
    const anim = shakeNode(box);
    return () => {
      anim?.cancel();
    };
  });

  // Строка сообщения: отступ 8 px + 13 px текста при line-height 1.45.
  const LINE = 27;

  const describedBy = $derived(
    [error ? `${id}-error` : null, hint ? `${id}-hint` : null].filter(Boolean).join(' ') ||
      undefined
  );
</script>

<div class="field {size}">
  <label for={id}>{label}</label>
  <div class="box" class:invalid={Boolean(error)} bind:this={box}>
    <input
      {id}
      bind:this={input}
      bind:value
      type={visible ? 'text' : 'password'}
      {autocomplete}
      spellcheck="false"
      required
      {disabled}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={describedBy}
    />
    <button
      type="button"
      class="toggle"
      aria-label={visible ? 'Скрыть пароль' : 'Показать пароль'}
      aria-pressed={visible}
      {disabled}
      onclick={() => {
        visible = !visible;
      }}
    >
      {#if visible}<EyeOff size={18} strokeWidth={1.75} aria-hidden="true" />{:else}<Eye
          size={18}
          strokeWidth={1.75}
          aria-hidden="true"
        />{/if}
    </button>
  </div>
  <div
    class="messages"
    style:min-height={reserveLines ? `${String(reserveLines * LINE)}px` : undefined}
  >
    {#if error}<p class="error" id="{id}-error" role="alert">{error}</p>{/if}
    {#if hint}<p class="hint" id="{id}-hint">{hint}</p>{/if}
  </div>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    width: 100%;
  }
  label {
    margin-bottom: var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .lg label {
    margin-bottom: var(--sp-2);
    color: inherit;
    font-size: var(--fs-13);
    font-weight: var(--fw-medium);
  }
  /* Рамка на контейнере: поле и кнопка — соседи, а не наложенные друг на друга элементы. */
  .box {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding-right: 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    transition:
      border-color var(--dur-fast) var(--ease-out),
      box-shadow var(--dur-fast) var(--ease-out);
  }
  .box:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  .box.invalid {
    border-color: var(--unpl-bar);
  }
  .box.invalid:focus-within {
    border-color: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-ring);
  }
  input {
    flex: 1;
    min-width: 0;
    height: calc(var(--h-control) - 2px);
    padding: 0 var(--sp-3);
    border: 0;
    background: transparent;
    color: var(--ink);
    font: var(--fw-regular) var(--fs-14) var(--font-sans);
    outline: none;
  }
  .lg input {
    height: calc(var(--h-control-lg) - 2px);
    font-size: 16px;
  }
  .toggle {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .lg .toggle {
    width: 36px;
    height: 36px;
  }
  .toggle:hover:not(:disabled) {
    background: var(--line-2);
    color: var(--ink);
  }
  .toggle:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
  }
  /* сообщение об ошибке появляется с лёгким сдвигом, а не рывком */
  .error {
    animation: error-in var(--dur-base) var(--ease-out);
    margin: var(--sp-2) 0 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
  .hint {
    margin: var(--sp-2) 0 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  @keyframes error-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>

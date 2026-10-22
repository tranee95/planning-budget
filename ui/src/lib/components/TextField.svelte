<script lang="ts">
  import { shake } from '$lib/motion/shake';
  type Props = {
    id: string;
    label: string;
    value?: string;
    error?: string | null;
    hint?: string | null;
    placeholder?: string;
    /** Моноширинный шрифт для кодов. */
    mono?: boolean;
    disabled?: boolean;
    /** Обязательное поле; необязательным (заметка, процент) подпись не нужна. */
    required?: boolean;
    /** `md` — формы внутри приложения (как `MoneyInput`); `lg` — экраны входа. */
    size?: 'md' | 'lg';
    /** Единица в конце поля («%»): часть поля, не часть значения. */
    suffix?: string;
    /** Значение зафиксировано: поле потеряло фокус или нажат Enter. */
    onchange?: () => void;
  };

  let {
    id,
    label,
    value = $bindable(''),
    error = null,
    hint = null,
    placeholder,
    mono = false,
    disabled = false,
    required = true,
    size = 'md',
    suffix,
    onchange
  }: Props = $props();

  let wrapEl = $state<HTMLDivElement>();
  let hadError = false;
  // Один раз трясётся, когда появляется ошибка
  $effect(() => {
    const has = Boolean(error);
    if (has && !hadError && wrapEl) shake(wrapEl);
    hadError = has;
  });

  const describedBy = $derived(
    [error ? `${id}-error` : null, hint ? `${id}-hint` : null].filter(Boolean).join(' ') ||
      undefined
  );
</script>

<div class="field {size}">
  <label for={id}>{label}</label>
  <div class="wrap" class:has-suffix={suffix !== undefined} bind:this={wrapEl}>
    <input
      {id}
      bind:value
      type="text"
      autocomplete="off"
      spellcheck="false"
      {placeholder}
      {required}
      {disabled}
      class:mono
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={describedBy}
      onchange={() => {
        onchange?.();
      }}
    />
    {#if suffix}<span class="suffix" aria-hidden="true">{suffix}</span>{/if}
  </div>
  {#if error}<p class="error" id="{id}-error" role="alert">{error}</p>{/if}
  {#if hint}<p class="hint" id="{id}-hint">{hint}</p>{/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    width: 100%;
  }
  .wrap {
    position: relative;
    display: flex;
  }
  .wrap input {
    width: 100%;
  }
  .has-suffix input {
    padding-inline-end: var(--sp-6);
  }
  .suffix {
    position: absolute;
    inset-block: 0;
    inset-inline-end: var(--sp-3);
    display: flex;
    align-items: center;
    color: var(--muted);
    font-size: var(--fs-14);
    pointer-events: none;
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
  input {
    height: var(--h-control);
    padding: 0 var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fw-regular) var(--fs-14) var(--font-sans);
    outline: none;
    transition:
      border-color var(--dur-fast) var(--ease-out),
      box-shadow var(--dur-fast) var(--ease-out);
  }
  .lg input {
    height: var(--h-control-lg);
    font-size: 16px;
  }
  input.mono {
    font-family: ui-monospace, 'Cascadia Mono', Consolas, monospace;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  /* заглавные буквы — только у введённого значения, подсказка остаётся как написана */
  input.mono::placeholder {
    text-transform: none;
    letter-spacing: normal;
  }
  input:focus-visible {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  input[aria-invalid='true'] {
    border-color: var(--unpl-bar);
  }
  input[aria-invalid='true']:focus-visible {
    border-color: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-ring);
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

<script lang="ts">
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
    size = 'md'
  }: Props = $props();

  const describedBy = $derived(
    [error ? `${id}-error` : null, hint ? `${id}-hint` : null].filter(Boolean).join(' ') ||
      undefined
  );
</script>

<div class="field {size}">
  <label for={id}>{label}</label>
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
  />
  {#if error}<p class="error" id="{id}-error" role="alert">{error}</p>{/if}
  {#if hint}<p class="hint" id="{id}-hint">{hint}</p>{/if}
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
  input:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  input[aria-invalid='true'] {
    border-color: var(--unpl-bar);
  }
  .error {
    margin: var(--sp-2) 0 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
  .hint {
    margin: var(--sp-2) 0 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
</style>

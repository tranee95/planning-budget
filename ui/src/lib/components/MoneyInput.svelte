<script lang="ts">
  import { shake } from '$lib/motion/shake';
  import {
    MAX_KOPECKS,
    checkMoney,
    formatMoneyField,
    groupMoneyInput,
    parseMoney
  } from '$lib/format';

  type Props = {
    id: string;
    label: string;
    placeholder?: string;
    /** Копейки; `null` — поле пусто. */
    value?: number | null;
    error?: string | null;
    size?: 'md' | 'lg';
    autofocus?: boolean;
    onenter?: (shift: boolean) => void;
  };

  let {
    id,
    label,
    placeholder,
    value = $bindable(null),
    error = null,
    size = 'md',
    autofocus = false,
    onenter
  }: Props = $props();

  let text = $state(value === null ? '' : toText(value));
  let focused = $state(false);
  let touched = $state(false);
  let boxEl = $state<HTMLDivElement>();
  let hadError = false;

  const check = $derived(text.trim() === '' ? null : checkMoney(text));
  const shownError = $derived(
    error ??
      (touched && check && !check.ok
        ? check.reason === 'range'
          ? 'Сумма не больше 999 999 999,99 ₽'
          : 'Рубли и не больше двух знаков после запятой'
        : null)
  );

  // Поле один раз трясётся в момент, когда появляется ошибка (не на каждое нажатие)
  $effect(() => {
    const has = shownError !== null;
    if (has && !hadError && boxEl) shake(boxEl);
    hadError = has;
  });

  function toText(kopecks: number): string {
    return formatMoneyField(kopecks);
  }

  // Значение изменили снаружи (сброс формы, undo): синхронизируем текст, пока пользователь не печатает.
  // Некорректный текст остаётся на экране вместе с подсказкой: значение у него null, как у пустого поля.
  $effect(() => {
    const typed = text.trim() === '' ? null : parseMoney(text);
    if (focused || typed === value) return;
    text = value === null ? '' : toText(value);
  });

  function onInput(event: Event & { currentTarget: HTMLInputElement }): void {
    const input = event.currentTarget;
    // Разряды ставятся на лету; курсор остаётся после того же числа цифр, что и до форматирования.
    const caret = input.selectionStart ?? input.value.length;
    const significant = input.value.slice(0, caret).replace(/\s/g, '').length;
    text = groupMoneyInput(input.value);
    if (text !== input.value) {
      input.value = text;
      let seen = 0;
      let pos = 0;
      while (pos < text.length && seen < significant) {
        if (!/\s/.test(text[pos] ?? '')) seen++;
        pos++;
      }
      input.setSelectionRange(pos, pos);
    }
    touched = true;
    value = text.trim() === '' ? null : parseMoney(text);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      const step = (event.shiftKey ? 1000 : 100) * 100;
      const next = Math.min(
        Math.max((value ?? 0) + (event.key === 'ArrowUp' ? step : -step), 0),
        MAX_KOPECKS
      );
      value = next;
      text = toText(next);
    } else if (event.key === 'Enter') {
      onenter?.(event.shiftKey);
    }
  }

  function onBlur(): void {
    focused = false;
    text = value === null ? text.trim() : toText(value);
  }
</script>

<div class="field {size}">
  <label for={id}>{label}</label>
  <div class="box" class:invalid={shownError !== null} bind:this={boxEl}>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      {id}
      class="num"
      type="text"
      inputmode="decimal"
      autocomplete="off"
      {autofocus}
      {placeholder}
      value={text}
      aria-invalid={shownError ? 'true' : undefined}
      aria-describedby={shownError ? `${id}-error` : undefined}
      oninput={onInput}
      onkeydown={onKeydown}
      onfocus={() => {
        focused = true;
      }}
      onblur={onBlur}
    />
    <span class="unit" aria-hidden="true">₽</span>
  </div>
  {#if shownError}<p class="error" id="{id}-error" role="alert">{shownError}</p>{/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }
  label {
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .box {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
  }
  .box:focus-within {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
  .box.invalid {
    border-color: var(--unpl);
  }
  .box.invalid:focus-within {
    border-color: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-ring);
  }
  input {
    flex: 1;
    min-width: 0;
    height: calc(var(--h-control) - 2px);
    border: 0;
    background: transparent;
    color: var(--ink);
    font: var(--fw-medium) var(--fs-14) var(--font-sans);
    font-variant-numeric: tabular-nums;
    outline: none;
  }
  .lg input {
    height: var(--h-control-lg);
    font-size: var(--fs-44);
    font-weight: var(--fw-semibold);
    letter-spacing: var(--tracking-tight);
  }
  .lg .box {
    align-items: baseline;
    padding-block: var(--sp-2);
  }
  /* крупная сумма: пропорциональные цифры, знак ₽ — 0,6 размера суммы на её базовой линии */
  .lg input {
    font-variant-numeric: proportional-nums;
  }
  .lg .unit {
    font-size: calc(var(--fs-44) * 0.6);
    font-weight: var(--fw-semibold);
  }
  .unit {
    color: var(--muted);
    font-size: var(--fs-17);
  }
  /* сообщение об ошибке появляется с лёгким сдвигом, а не рывком */
  .error {
    animation: error-in var(--dur-base) var(--ease-out);
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-12);
  }
  @keyframes error-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>

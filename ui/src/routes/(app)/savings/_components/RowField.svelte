<script lang="ts">
  import { formatMoneyTrim, parseMoney } from '$lib/format';

  type Props = {
    /** Доступное имя поля: видимой подписи в строке нет. */
    label: string;
    kind?: 'text' | 'money';
    placeholder?: string;
    /** Текст для `text`, копейки (`null` — пусто) для `money`. */
    value?: string | number | null;
    invalid?: boolean;
    /** Описание ошибки для скринридера (id элемента с текстом). */
    describedby?: string;
    onenter?: () => void;
    onescape?: () => void;
    el?: HTMLInputElement;
  };

  let {
    label,
    kind = 'text',
    placeholder,
    value = $bindable(kind === 'money' ? null : ''),
    invalid = false,
    describedby,
    onenter,
    onescape,
    el = $bindable()
  }: Props = $props();

  let focused = $state(false);
  let text = $state(initial());

  function initial(): string {
    if (kind === 'text') return typeof value === 'string' ? value : '';
    return typeof value === 'number' ? shown(value) : '';
  }

  /** Вне фокуса сумма показана тем же форматтером, что и в тексте, но без «₽»: он в суффиксе. */
  function shown(kopecks: number): string {
    return formatMoneyTrim(kopecks).replace(/\s?₽$/, '');
  }

  // Значение изменили снаружи (сброс после добавления, undo): текст следует за ним, в том числе
  // когда в поле стоит курсор. Пока печатают, значение всегда равно разбору текста, и текст не трогается.
  $effect(() => {
    const typed = kind === 'text' ? text : text.trim() === '' ? null : parseMoney(text);
    if (typed === value) return;
    if (kind === 'text') text = String(value ?? '');
    else if (typeof value !== 'number') text = '';
    else text = focused ? shown(value).replace(/\s/g, '') : shown(value);
  });

  function oninput(event: Event & { currentTarget: HTMLInputElement }): void {
    text = event.currentTarget.value;
    value = kind === 'text' ? text : text.trim() === '' ? null : parseMoney(text);
  }

  function onfocus(event: FocusEvent & { currentTarget: HTMLInputElement }): void {
    focused = true;
    // При фокусе — сырое число для правки, без разделителей разрядов.
    if (kind === 'money' && typeof value === 'number') {
      text = shown(value).replace(/\s/g, '');
      event.currentTarget.select();
    }
  }

  function onblur(): void {
    focused = false;
    if (kind === 'money') text = typeof value === 'number' ? shown(value) : text.trim();
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      onenter?.();
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      onescape?.();
    }
  }
</script>

<div class="box" class:invalid class:money={kind === 'money'}>
  <input
    bind:this={el}
    class:num={kind === 'money'}
    type="text"
    inputmode={kind === 'money' ? 'decimal' : undefined}
    autocomplete="off"
    aria-label={label}
    aria-invalid={invalid ? 'true' : undefined}
    aria-describedby={invalid ? describedby : undefined}
    {placeholder}
    value={text}
    {oninput}
    {onfocus}
    {onblur}
    {onkeydown}
  />
  {#if kind === 'money'}<span class="unit" aria-hidden="true">₽</span>{/if}
</div>

<style>
  .box {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
  }
  .box:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .box.invalid {
    border-color: var(--unpl);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--ink);
    font: var(--fw-medium) var(--fs-14) var(--font-sans);
  }
  input::placeholder {
    color: var(--muted);
    font-weight: var(--fw-regular, 400);
  }
  .money input {
    text-align: right;
  }
  .unit {
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>

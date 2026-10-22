<script lang="ts">
  import { dateFieldToIso, isoToDateField, maskDateInput } from '$lib/format';

  type Props = {
    id: string;
    /** Дата `ГГГГ-ММ-ДД`; пустая строка — дата не задана или набрана не до конца. */
    value?: string;
    /** Что набрано: пусто или полная существующая дата. Родитель не сохраняет, пока `false`. */
    valid?: boolean;
    invalid?: boolean;
    describedby?: string;
    onenter?: (shift: boolean) => void;
  };

  let {
    id,
    value = $bindable(''),
    valid = $bindable(true),
    invalid = false,
    describedby,
    onenter
  }: Props = $props();

  let text = $state(isoToDateField(value));
  let focused = $state(false);

  // Значение изменили снаружи (сброс формы, undo), пока поле не в фокусе.
  $effect(() => {
    if (!focused && (dateFieldToIso(text) ?? '') !== value) text = isoToDateField(value);
  });

  function onInput(event: Event & { currentTarget: HTMLInputElement }): void {
    const input = event.currentTarget;
    const typed = input.value;
    let next = maskDateInput(typed);
    // при стирании точка, поставленная маской, не возвращается: иначе день и месяц нельзя стереть
    if (typed.length < text.length && next.endsWith('.')) next = next.slice(0, -1);
    text = next;
    input.value = text;
    value = dateFieldToIso(text) ?? '';
    valid = text === '' || value !== '';
  }
</script>

<!-- Своё поле вместо нативного date: подсказка «дд.мм.гггг» по-русски, а не по языку ОС -->
<input
  {id}
  class="date"
  type="text"
  inputmode="numeric"
  autocomplete="off"
  placeholder="дд.мм.гггг"
  maxlength="10"
  value={text}
  aria-invalid={invalid || !valid ? 'true' : undefined}
  aria-describedby={describedby}
  oninput={onInput}
  onfocus={() => {
    focused = true;
  }}
  onblur={() => {
    focused = false;
  }}
  onkeydown={(event) => {
    if (event.key === 'Enter') onenter?.(event.shiftKey);
  }}
/>

<style>
  .date {
    box-sizing: border-box;
    width: 100%;
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-14);
    font-variant-numeric: tabular-nums;
  }
  .date[aria-invalid='true'] {
    border-color: var(--unpl);
  }
  .date[aria-invalid='true']:focus-visible {
    border-color: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-ring);
  }
  .date:focus-visible {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px var(--focus-ring);
  }
</style>

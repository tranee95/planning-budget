<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import type { QueryHintSpanDto, TokenSpanDto } from '$lib/api/bindings';
  import { HINT_TEXT, KIND_LABEL, removeSpan, sliceChars } from '$lib/palette/query';

  type Props = {
    /** Строка, для которой посчитаны `spans` и `hints`: по ней режутся токены. */
    query: string;
    spans: readonly TokenSpanDto[];
    hints: readonly QueryHintSpanDto[];
    onremove: (nextQuery: string) => void;
  };

  let { query, spans, hints: hintSpans, onremove }: Props = $props();

  const chips = $derived(
    spans.map((span) => {
      const raw = sliceChars(query, span.start, span.end);
      return {
        key: `${String(span.start)}-${String(span.end)}`,
        kind: span.kind,
        text: span.kind === 'text' ? `текст: «${raw}»` : raw,
        label: `Убрать ${KIND_LABEL[span.kind]} ${raw}`,
        start: span.start,
        end: span.end
      };
    })
  );
  const hints = $derived(
    hintSpans.map((h) => ({
      key: `${String(h.start)}-${String(h.end)}`,
      token: sliceChars(query, h.start, h.end),
      text: HINT_TEXT[h.hint]
    }))
  );
</script>

{#if chips.length > 0}
  <ul class="chips" aria-label="Распознанные фильтры">
    {#each chips as chip (chip.key)}
      <li class="chip {chip.kind}">
        <span>{chip.text}</span>
        <button
          type="button"
          tabindex="-1"
          aria-label={chip.label}
          onclick={() => {
            onremove(removeSpan(query, chip.start, chip.end));
          }}
        >
          <X size={12} aria-hidden="true" />
        </button>
      </li>
    {/each}
  </ul>
{/if}
{#each hints as hint (hint.key)}
  <p class="hint"><b>{hint.token}</b>: {hint.text}</p>
{/each}

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    margin: 0;
    padding: var(--sp-3) var(--sp-5) var(--sp-1);
    list-style: none;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
    height: 24px;
    padding-inline: var(--sp-3) var(--sp-1);
    border: 1px solid var(--line);
    border-radius: var(--r-full);
    background: var(--surface-2);
    color: var(--ink-2);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .chip.text {
    border-color: transparent;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .chip button {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: var(--r-full);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .chip button:hover {
    background: var(--line-2);
  }
  .hint {
    margin: 0;
    padding: var(--sp-1) var(--sp-5);
    color: var(--muted);
    font-size: var(--fs-12);
  }
</style>

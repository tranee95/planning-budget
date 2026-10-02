<script lang="ts" generics="T extends string">
  type Props = {
    id: string;
    label: string;
    options: readonly { value: T; label: string }[];
    value: T;
    onchange: (value: T) => void;
    disabled?: boolean;
    /** Значения, которые сейчас выбрать нельзя, и причина (показывается подсказкой). */
    blocked?: (value: T) => string | null;
  };

  let { id, label, options, value, onchange, disabled = false, blocked }: Props = $props();
</script>

<div class="field">
  <label for={id}>{label}</label>
  <select
    {id}
    {value}
    {disabled}
    onchange={(event) => {
      const picked = options.find((o) => o.value === event.currentTarget.value);
      if (picked) onchange(picked.value);
    }}
  >
    {#each options as option (option.value)}
      <option
        value={option.value}
        disabled={blocked?.(option.value) != null}
        title={blocked?.(option.value) ?? undefined}>{option.label}</option
      >
    {/each}
  </select>
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
  select {
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fs-14) var(--font-sans);
  }
  select:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  select:disabled {
    opacity: 0.45;
  }
</style>

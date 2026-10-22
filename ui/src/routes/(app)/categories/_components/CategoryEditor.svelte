<script lang="ts">
  import { PLACEHOLDERS } from '$lib/i18n/placeholders';
  import type { CategoryKindDto } from '$lib/api/bindings';
  import Check from '@lucide/svelte/icons/check';
  import { Button, MoneyInput, Segmented, Sheet, TextField } from '$lib/components';
  import { CATEGORY_COLORS } from '$lib/category-colors';
  import { formatMoney, formatMonthGenitive } from '$lib/format';
  import { getCategoriesVm } from '../categories.svelte';

  const vm = getCategoriesVm();

  const KINDS = [
    { value: 'mandatory', label: 'Обязательные' },
    { value: 'wants', label: 'Желания' },
    { value: 'loans', label: 'Займы' }
  ] as const;

  const KIND_LABEL: Record<CategoryKindDto, string> = {
    ...Object.fromEntries(KINDS.map((k) => [k.value, k.label])),
    savings: 'Сбережения'
  } as Record<CategoryKindDto, string>;

  /** Десять цветов палитры; цвет, выбранный раньше и вышедший из палитры, остаётся в списке и отмечен. */
  const palette = $derived(
    CATEGORY_COLORS.some((c) => c.toLowerCase() === vm.draft.color.toLowerCase())
      ? CATEGORY_COLORS
      : [...CATEGORY_COLORS, vm.draft.color]
  );
  const isNew = $derived(vm.selected === 'new');
  const row = $derived(vm.rows.find((r) => r.category.id === vm.selected) ?? null);
  const title = $derived(isNew ? 'Новая категория' : (vm.current?.name ?? ''));
  /** «Экономия при лимите» = среднее − лимит, если среднее выше. */
  const saving = $derived(
    row !== null && row.overLimit !== null && row.overLimit > 0 ? row.overLimit : null
  );
</script>

<Sheet
  {title}
  width={400}
  onclose={() => {
    vm.closeEditor();
  }}
>
  <form
    id="cat-form"
    onsubmit={(e) => {
      e.preventDefault();
      void vm.save();
    }}
  >
    <TextField
      id="cat-name"
      label="Название"
      placeholder={PLACEHOLDERS.categoryName}
      bind:value={vm.draft.name}
    />

    <div class="group">
      <span class="lbl" id="cat-color-label">Цвет</span>
      <div class="colors" role="radiogroup" aria-labelledby="cat-color-label">
        {#each palette as color (color)}
          <button
            type="button"
            role="radio"
            aria-checked={vm.draft.color === color}
            aria-label="Цвет {color}"
            class="color"
            class:on={vm.draft.color === color}
            style:background={color}
            onclick={() => {
              vm.draft.color = color;
            }}
            >{#if vm.draft.color === color}<Check size={16} aria-hidden="true" />{/if}</button
          >
        {/each}
      </div>
    </div>

    <div class="group">
      <span class="lbl">Тип</span>
      {#if isNew}
        <Segmented
          label="Тип категории"
          options={KINDS}
          value={vm.draft.kind}
          onchange={(kind: CategoryKindDto) => {
            vm.draft.kind = kind;
          }}
        />
      {:else}
        <span class="kind">{KIND_LABEL[vm.draft.kind]}</span>
        <span class="hint">Тип задаётся при создании категории и не меняется.</span>
      {/if}
    </div>

    <div class="group">
      <MoneyInput
        id="cat-limit"
        label="Лимит в месяц"
        placeholder={PLACEHOLDERS.categoryLimit}
        bind:value={vm.draft.limit}
      />
      <span class="hint">
        Действует с {formatMonthGenitive(vm.month)}. Прошлые месяцы сохранят свои лимиты.
      </span>
    </div>

    <TextField
      id="cat-note"
      label="Обоснование"
      placeholder={PLACEHOLDERS.categoryNote}
      required={false}
      bind:value={vm.draft.note}
      hint="Почему такой лимит: видно только вам."
    />

    {#if row !== null}
      <dl class="stats">
        <div>
          <dt>Среднее за год</dt>
          <dd class="num">{row.avg === 0 ? '—' : formatMoney(row.avg)}</dd>
        </div>
        {#if saving !== null}
          <div>
            <dt>Экономия при лимите</dt>
            <dd class="num good">{formatMoney(saving)} / мес</dd>
          </div>
        {/if}
      </dl>
    {/if}

    {#if vm.fieldError}
      <p class="error" role="alert">{vm.fieldError}</p>
    {/if}
  </form>
  {#snippet footer()}
    {#if !isNew && vm.current}
      {@const archived = vm.current.archived}
      {@const id = vm.current.id}
      <Button
        onclick={() => {
          if (archived) void vm.setArchived(id, false);
          else vm.askArchive(id);
        }}>{archived ? 'Вернуть из архива' : 'В архив'}</Button
      >
    {:else}
      <Button
        onclick={() => {
          vm.closeEditor();
        }}>Отмена</Button
      >
    {/if}
    <Button type="submit" form="cat-form" variant="primary" loading={vm.saving}>Сохранить</Button>
  {/snippet}
</Sheet>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .lbl {
    color: var(--muted);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  /* 5 × 2: десятый цвет не переносится на третью строку */
  .colors {
    display: grid;
    grid-template-columns: repeat(5, 32px);
    gap: var(--sp-3);
  }
  .color {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    color: var(--accent-ink);
    cursor: pointer;
  }
  /* выбранный цвет: кольцо и галочка */
  .color.on {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }
  .color:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--primary);
  }
  .kind {
    font-size: var(--fs-14);
  }
  .hint {
    color: var(--muted);
    font-size: var(--fs-12);
    line-height: 1.5;
  }
  .stats {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    margin: 0;
    padding: var(--sp-3) var(--sp-4);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface-2);
    font-size: var(--fs-13);
  }
  .stats div {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-2);
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-weight: var(--fw-semibold);
  }
  .good {
    color: var(--paid);
  }
  .error {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

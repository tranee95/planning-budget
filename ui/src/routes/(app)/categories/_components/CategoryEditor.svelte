<script lang="ts">
  import type { CategoryKindDto } from '$lib/api/bindings';
  import { Button, MoneyInput, Segmented, TextField } from '$lib/components';
  import { CATEGORY_COLORS } from '$lib/category-colors';
  import { formatMoney, formatMonth, formatPercent } from '$lib/format';
  import { pop } from '$lib/motion';
  import { getCategoriesVm } from '../categories.svelte';

  const vm = getCategoriesVm();

  const KINDS = [
    { value: 'mandatory', label: 'Обязательные' },
    { value: 'wants', label: 'Желания' },
    { value: 'loans', label: 'Займы' },
    { value: 'savings', label: 'Сбережения' }
  ] as const;

  const KIND_LABEL = Object.fromEntries(KINDS.map((k) => [k.value, k.label])) as Record<
    CategoryKindDto,
    string
  >;

  const isNew = $derived(vm.selected === 'new');
  const row = $derived(vm.rows.find((r) => r.category.id === vm.selected) ?? null);
  const isSavings = $derived(vm.draft.kind === 'savings');
  const title = $derived(isNew ? 'Новая категория' : (vm.current?.name ?? ''));
  /** «Экономия при лимите» = среднее − лимит, если среднее выше. */
  const saving = $derived(
    row !== null && row.overLimit !== null && row.overLimit > 0 ? row.overLimit : null
  );
</script>

<aside class="editor" aria-label="Редактирование категории" in:pop>
  <h2>{title}</h2>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void vm.save();
    }}
  >
    <TextField id="cat-name" label="Название" bind:value={vm.draft.name} />

    <div class="group">
      <span class="lbl" id="cat-color-label">Цвет</span>
      <div class="colors" role="radiogroup" aria-labelledby="cat-color-label">
        {#each CATEGORY_COLORS as color (color)}
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
          ></button>
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
      {/if}
    </div>

    {#if isSavings}
      <TextField
        id="cat-rate"
        label="Процент плана от дохода"
        required={false}
        bind:value={vm.draft.rate}
        placeholder="14"
        hint="Действует с {formatMonth(vm.month)}. Прошлые месяцы сохранят свой процент."
      />
      <TextField
        id="cat-month-rate"
        label="Процент только на {formatMonth(vm.month)}"
        required={false}
        bind:value={vm.draft.monthRate}
        placeholder="не меняется"
        hint={vm.monthPlanBp === null
          ? null
          : `План месяца: ${formatPercent(vm.monthPlanBp)} дохода`}
      />
      {#if vm.current}
        {@const id = vm.current.id}
        <Button
          size="sm"
          onclick={() => {
            void vm.clearMonthRate(id);
          }}>Сбросить процент месяца</Button
        >
      {/if}
    {:else}
      <div class="group">
        <MoneyInput id="cat-limit" label="Лимит в месяц" bind:value={vm.draft.limit} />
        <span class="hint">
          Действует с {formatMonth(vm.month)}. Прошлые месяцы сохранят свои лимиты.
        </span>
      </div>
    {/if}

    <TextField
      id="cat-note"
      label="Обоснование"
      required={false}
      bind:value={vm.draft.note}
      hint="Почему такой лимит: видно только вам."
    />

    {#if row !== null && !isSavings}
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

    <div class="actions">
      {#if !isNew && vm.current}
        {@const archived = vm.current.archived}
        {@const id = vm.current.id}
        <Button
          onclick={() => {
            void vm.setArchived(id, !archived);
          }}>{archived ? 'Вернуть из архива' : 'В архив'}</Button
        >
      {:else}
        <Button
          onclick={() => {
            vm.closeEditor();
          }}>Отмена</Button
        >
      {/if}
      <Button type="submit" variant="primary" loading={vm.saving}>Сохранить</Button>
    </div>
  </form>
</aside>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    width: 360px;
    flex-shrink: 0;
    padding: var(--sp-6);
    border: 1px solid var(--line);
    border-radius: var(--r-xl);
    background: var(--surface);
    box-shadow: var(--shadow);
    overflow-y: auto;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: var(--fw-semibold);
  }
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
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }
  .color {
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .color.on {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }
  .color:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
  .actions {
    display: flex;
    gap: var(--sp-3);
  }
  .actions :global(button) {
    flex: 1;
  }
</style>

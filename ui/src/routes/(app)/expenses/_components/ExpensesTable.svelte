<script lang="ts">
  import { rise } from '$lib/motion';
  import type { TransactionDto } from '$lib/api/bindings';
  import { Button, EmptyState, Select, StatusChip } from '$lib/components';
  import DataTable from '$lib/components/DataTable.svelte';
  import { STATUS_LABEL, STATUS_ORDER } from '$lib/components/status';
  import type { Column } from '$lib/components/table';
  import { formatDay, formatMoney } from '$lib/format';
  import { categories } from '$lib/stores/categories.svelte';
  import { formatMonth } from '$lib/format';
  import { tags } from '$lib/stores/tags.svelte';
  import { getExpensesVm, type SortColumn } from '../expenses.svelte';
  import InlineEdit from './InlineEdit.svelte';
  import TagPicker from './TagPicker.svelte';

  const vm = getExpensesVm();

  const COLUMNS: readonly Column[] = [
    { id: 'check', label: '', width: '32px' },
    { id: 'date', label: 'Дата', width: '72px', sortable: true },
    { id: 'month', label: 'Месяц', width: '120px', sortable: true },
    { id: 'title', label: 'Наименование', width: '1fr', sortable: true },
    { id: 'category', label: 'Категория', width: '180px', sortable: true },
    { id: 'tags', label: 'Теги', width: '150px' },
    { id: 'amount', label: 'Сумма', width: '120px', align: 'end', sortable: true },
    { id: 'status', label: 'Статус', width: '150px', sortable: true }
  ];

  // Месяц нужен, когда строки приходят из нескольких месяцев сразу.
  const columns = $derived(vm.filtered ? COLUMNS : COLUMNS.filter((c) => c.id !== 'month'));
  const tagNames = (ids: readonly number[]): string =>
    ids
      .map((id) => tags.byId.get(id)?.name ?? '')
      .filter((n) => n !== '')
      .join(', ');
  const tagEditRow = $derived(vm.transactions.find((t) => t.id === vm.tagEditId));

  const categoryNames = $derived(new Map(categories.items.map((c) => [c.id, c.name])));
  const categoryOptions = $derived([
    { value: '', label: 'Перенести в категорию…' },
    ...categories.items.map((c) => ({ value: String(c.id), label: c.name }))
  ]);
  const allChecked = $derived(
    vm.visible.length > 0 && vm.visible.every((t) => vm.checked.has(t.id))
  );
</script>

<div class="wrap" in:rise>
  <div class="bar">
    <label class="all">
      <input
        type="checkbox"
        checked={allChecked}
        disabled={vm.visible.length === 0}
        onchange={() => {
          vm.toggleAllChecked();
        }}
      />
      Выбрать все ({vm.visible.length})
    </label>
    {#if vm.checkedVisible.length > 0}
      <div class="actions" role="toolbar" aria-label="Действия с выбранными">
        <span class="count">Выбрано {vm.checkedVisible.length}</span>
        {#each STATUS_ORDER as status (status)}
          <Button
            variant="secondary"
            size="sm"
            onclick={() => {
              void vm.bulkSetStatus(status);
            }}>{STATUS_LABEL[status]}</Button
          >
        {/each}
        <Select
          id="bulk-category"
          label="Категория"
          options={categoryOptions}
          value=""
          onchange={(value: string) => {
            if (value !== '') void vm.bulkSetCategory(Number(value));
          }}
        />
        <Button
          variant="danger"
          size="sm"
          onclick={() => {
            void vm.bulkDelete();
          }}>Удалить</Button
        >
        <Button
          variant="ghost"
          size="sm"
          onclick={() => {
            vm.clearChecked();
          }}>Снять выбор</Button
        >
      </div>
    {/if}
  </div>

  <div class="table">
    <DataTable
      {columns}
      rows={vm.sortedRows}
      rowKey={(row: TransactionDto) => row.id}
      label="Траты месяца"
      sort={vm.sort}
      onsort={(column: string) => {
        vm.toggleSort(column as SortColumn);
      }}
      selected={vm.checked}
      onselect={(key: number | string) => {
        vm.toggleChecked(Number(key));
      }}
      onopen={(row: TransactionDto) => {
        vm.startEdit(row.id);
      }}
    >
      {#snippet cell(row: TransactionDto, column: Column)}
        {#if column.id === 'check'}
          <label class="box">
            <input
              type="checkbox"
              aria-label="Выбрать «{row.title}»"
              checked={vm.checked.has(row.id)}
              onchange={() => {
                vm.toggleChecked(row.id);
              }}
            />
          </label>
        {:else if column.id === 'date'}
          <span class="muted">{formatDay(row.date)}</span>
        {:else if column.id === 'month'}
          <span class="muted">{formatMonth(row.month, 'short')}</span>
        {:else if column.id === 'title'}
          {#if vm.editingId === row.id}
            <InlineEdit
              {row}
              onsave={(patch: { title: string; amount: number }) => {
                void vm.saveEdit(patch);
              }}
              oncancel={() => {
                vm.cancelEdit();
              }}
            />
          {:else}
            <button
              type="button"
              class="link"
              onclick={() => {
                vm.startEdit(row.id);
              }}>{row.title}</button
            >
          {/if}
        {:else if column.id === 'category'}
          {categoryNames.get(row.categoryId) ?? '—'}
        {:else if column.id === 'tags'}
          <button
            type="button"
            class="link tags"
            title={tagNames(row.tagIds)}
            aria-label="Теги: {tagNames(row.tagIds) || 'нет'}. Изменить"
            onclick={() => {
              vm.tagEditId = row.id;
            }}>{tagNames(row.tagIds) || '—'}</button
          >
        {:else if column.id === 'amount'}
          {#if vm.editingId !== row.id}{formatMoney(row.amount)}{/if}
        {:else}
          <button
            type="button"
            class="link"
            aria-label="{STATUS_LABEL[row.status]}. Сменить статус"
            onclick={() => {
              void vm.cycleStatus(row.id);
            }}><StatusChip status={row.status} /></button
          >
        {/if}
      {/snippet}
      {#snippet empty()}
        {#if vm.filtered}
          <EmptyState title="Ничего не найдено" hint="Измените запрос или уберите лишние чипы." />
        {:else}
          <EmptyState title="В этом месяце нет трат" hint="Добавьте первую в режиме «Блоки»." />
        {/if}
      {/snippet}
    </DataTable>
  </div>
</div>

{#if tagEditRow}
  <TagPicker
    row={tagEditRow}
    onsave={(ids: number[]) => {
      void vm.setTags(tagEditRow.id, ids);
    }}
    onclose={() => {
      vm.tagEditId = null;
    }}
  />
{/if}

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-height: 0;
  }
  .table {
    display: flex;
    flex-direction: column;
    height: min(640px, calc(100vh - 260px));
    min-height: 240px;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-4);
    min-height: 56px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
  }
  .count {
    margin-right: var(--sp-2);
    font-size: var(--fs-13);
    font-weight: var(--fw-medium);
  }
  .muted {
    color: var(--muted);
  }
  .tags {
    min-width: 24px;
    max-width: 100%;
    overflow: hidden;
    color: var(--muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .box {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    cursor: pointer;
  }
  .all {
    display: flex;
    align-items: center;
    min-height: 24px;
    gap: var(--sp-2);
    color: var(--muted);
    font-size: var(--fs-13);
  }
</style>

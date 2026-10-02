<script lang="ts">
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import { Button, IconButton, Modal, Tabs, TextField } from '$lib/components';
  import { getAnalyticsVm } from '../analytics.svelte';
  import { getBuilderVm } from '../builder.svelte';

  const vm = getAnalyticsVm();
  const builder = getBuilderVm();

  type Dialog = { kind: 'create' } | { kind: 'rename' } | { kind: 'delete' } | null;

  let dialog = $state<Dialog>(null);
  let name = $state('');
  let busy = $state(false);

  const tabs = $derived(vm.dashboards.map((d) => ({ value: String(d.id), label: d.name })));
  const only = $derived(vm.dashboards.length <= 1);

  function open(next: NonNullable<Dialog>): void {
    name = next.kind === 'rename' ? (vm.active?.name ?? '') : '';
    dialog = next;
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (dialog === null || busy) return;
    busy = true;
    const ok =
      dialog.kind === 'create'
        ? await vm.createDashboard(name)
        : dialog.kind === 'rename'
          ? await vm.renameDashboard(vm.activeId ?? 0, name)
          : (await vm.deleteDashboard(vm.activeId ?? 0), true);
    busy = false;
    if (ok) dialog = null;
  }
</script>

<div class="bar">
  <Tabs
    label="Дашборды"
    {tabs}
    value={String(vm.activeId ?? '')}
    onchange={(value: string) => {
      void vm.select(Number(value));
    }}
  />
  <IconButton
    size="sm"
    aria-label="Новый дашборд"
    onclick={() => {
      open({ kind: 'create' });
    }}
  >
    <Plus size={16} strokeWidth={1.75} aria-hidden="true" />
  </IconButton>
  <span class="spacer"></span>
  <Button
    variant="primary"
    size="sm"
    disabled={vm.active === null}
    onclick={() => {
      builder.beginCreate();
    }}>График</Button
  >
  <IconButton
    size="sm"
    aria-label="Переименовать дашборд"
    disabled={vm.active === null}
    onclick={() => {
      open({ kind: 'rename' });
    }}
  >
    <Pencil size={16} strokeWidth={1.75} aria-hidden="true" />
  </IconButton>
  <IconButton
    size="sm"
    aria-label="Удалить дашборд"
    disabled={only}
    title={only ? 'Последний дашборд удалить нельзя' : undefined}
    onclick={() => {
      open({ kind: 'delete' });
    }}
  >
    <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
  </IconButton>
</div>

{#if dialog !== null}
  <Modal
    title={dialog.kind === 'create'
      ? 'Новый дашборд'
      : dialog.kind === 'rename'
        ? 'Переименовать дашборд'
        : 'Удалить дашборд'}
    width={440}
    onclose={() => {
      dialog = null;
    }}
  >
    <form id="dashboard-form" onsubmit={submit}>
      {#if dialog.kind === 'delete'}
        <p>
          Дашборд «{vm.active?.name}» и все его графики будут удалены. Данные бюджета не изменятся.
        </p>
      {:else}
        <TextField id="dashboard-name" label="Название" bind:value={name} />
      {/if}
    </form>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          dialog = null;
        }}>Отмена</Button
      >
      <Button
        type="submit"
        form="dashboard-form"
        variant={dialog?.kind === 'delete' ? 'danger' : 'primary'}
        loading={busy}
        >{dialog?.kind === 'create'
          ? 'Создать'
          : dialog?.kind === 'rename'
            ? 'Сохранить'
            : 'Удалить'}</Button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .spacer {
    flex: 1;
  }
  p {
    margin: 0;
    color: var(--ink-2);
  }
</style>

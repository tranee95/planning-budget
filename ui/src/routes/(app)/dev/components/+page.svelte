<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import { SvelteSet } from 'svelte/reactivity';
  import type { ChartDataDto, ChartSpecDto, TxStatusDto } from '$lib/api/bindings';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import Chart from '$lib/components/Chart.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Combobox from '$lib/components/Combobox.svelte';
  import CorridorBar from '$lib/components/CorridorBar.svelte';
  import DataTable from '$lib/components/DataTable.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Kbd from '$lib/components/Kbd.svelte';
  import Kpi from '$lib/components/Kpi.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import MoneyInput from '$lib/components/MoneyInput.svelte';
  import Progress from '$lib/components/Progress.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import Select from '$lib/components/Select.svelte';
  import Sheet from '$lib/components/Sheet.svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { STATUS_ORDER } from '$lib/components/status';
  import StatusChip from '$lib/components/StatusChip.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import StatusSegmented from '$lib/components/StatusSegmented.svelte';
  import Switch from '$lib/components/Switch.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import type { Column } from '$lib/components/table';
  import TextField from '$lib/components/TextField.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { formatMoney } from '$lib/format';
  import { toasts } from '$lib/stores/toasts.svelte';

  const chartSpec = (patch: Partial<ChartSpecDto>): ChartSpecDto => ({
    version: 1,
    title: 'Пример графика',
    type: 'bar',
    metric: 'spent',
    groupBy: 'month',
    seriesBy: null,
    metrics: [],
    period: { preset: 'ytd' },
    filter: '',
    options: {
      showLimit: false,
      topN: null,
      sort: 'natural',
      cumulative: false,
      comparePrevPeriod: false,
      percent: false
    },
    ...patch
  });
  const months = ['Июль', 'Август', 'Сентябрь'];
  const stackedData: ChartDataDto = {
    categories: months,
    categoryTokens: ['month:2026-07', 'month:2026-08', 'month:2026-09'],
    series: [
      { name: 'Оплачено', color: 'status.paid', values: [90000, 110000, 120000] },
      { name: 'Долг', color: 'status.debt', values: [8000, 0, 12000] },
      { name: 'План', color: 'status.planned', values: [0, 15000, 17000] }
    ],
    referenceLines: [],
    totals: [98000, 125000, 149000],
    unit: 'rub'
  };
  const rateData: ChartDataDto = {
    categories: months,
    categoryTokens: ['', '', ''],
    series: [
      { name: 'Норма сбережений', color: 'metric.savings_rate', values: [13.7, 13.1, 12.6] }
    ],
    referenceLines: [{ name: 'Коридор сбережений', from: 13, to: 15 }],
    totals: null,
    unit: 'percent'
  };
  const donutData: ChartDataDto = {
    categories: ['Обязательные', 'Желания', 'Сбережения'],
    categoryTokens: ['kind.mandatory', 'kind.wants', 'kind.savings'],
    series: [{ name: 'Траты', color: 'metric.spent', values: [74200, 68500, 29400] }],
    referenceLines: [],
    totals: [74200, 68500, 29400],
    unit: 'rub'
  };

  type Row = { id: number; name: string; status: TxStatusDto; amount: number };
  type Sort = { column: string; descending: boolean };

  let status = $state<TxStatusDto>('paid');
  let amount = $state<number | null>(259650);
  let tab = $state('blocks');
  let size = $state('md');
  let category = $state<string | null>(null);
  let pick = $state('b');
  let on = $state(true);
  let checked = $state(false);
  let text = $state('');
  let modal = $state(false);
  let longModal = $state(false);
  let sheet = $state(false);
  let sort = $state<Sort | null>(null);
  const selected = new SvelteSet<number | string>();

  const categories = [
    { value: 'food', label: 'Продукты' },
    { value: 'home', label: 'Дом' },
    { value: 'fun', label: 'Развлечения' }
  ];

  const columns: Column[] = [
    { id: 'name', label: 'Наименование', width: '1fr', sortable: true },
    { id: 'status', label: 'Статус', width: '160px' },
    { id: 'amount', label: 'Сумма', width: '140px', align: 'end', sortable: true }
  ];

  const all: Row[] = Array.from({ length: 5000 }, (_, i) => ({
    id: i,
    name: `Трата ${String(i + 1)}`,
    status: STATUS_ORDER[i % 4] ?? 'paid',
    amount: ((i * 7919) % 90000) * 100 + 10000
  }));

  const rows = $derived.by(() => {
    const by = sort;
    if (!by) return all;
    const dir = by.descending ? -1 : 1;
    return [...all].sort((a, b) =>
      by.column === 'amount'
        ? (a.amount - b.amount) * dir
        : a.name.localeCompare(b.name, 'ru', { numeric: true }) * dir
    );
  });

  function toggleSort(column: string): void {
    sort = { column, descending: sort?.column === column && !sort.descending };
  }

  function toggleRow(key: number | string): void {
    if (selected.has(key)) selected.delete(key);
    else selected.add(key);
  }
</script>

<svelte:head>
  <title>Private Budget — компоненты</title>
</svelte:head>

<div class="grid">
  <Card title="Кнопки">
    <div class="row">
      <Button variant="primary">Основная</Button>
      <Button>Вторичная</Button>
      <Button variant="ghost">Призрачная</Button>
      <Button variant="danger">Опасная</Button>
      <Button variant="primary" loading>Загрузка</Button>
      <Button disabled>Отключена</Button>
      <IconButton aria-label="Добавить"><Plus size={18} aria-hidden="true" /></IconButton>
      <Tooltip text="Новая трата" shortcut="N">
        <IconButton aria-label="Новая трата"><Plus size={18} aria-hidden="true" /></IconButton>
      </Tooltip>
      <Kbd>Ctrl K</Kbd>
    </div>
  </Card>

  <Card title="Статусы">
    <div class="row">
      {#each STATUS_ORDER as s (s)}
        <StatusDot status={s} />
        <StatusChip status={s} />
      {/each}
    </div>
    <StatusSegmented
      value={status}
      onchange={(v: TxStatusDto) => {
        status = v;
      }}
    />
  </Card>

  <Card title="Прогресс и коридор">
    <Progress value={0.4} label="Лимит" />
    <Progress value={0.7} planned={0.2} label="Лимит с планом" />
    <Progress value={1.3} label="Превышение" />
    <CorridorBar value={1100} min={1300} max={1500} />
    <CorridorBar value={1400} min={1300} max={1500} />
  </Card>

  <Card title="KPI">
    <div class="row">
      <Kpi label="Расходы" value={formatMoney(8_650_000)} hint="−8% к среднему" />
      <Kpi label="Свободный остаток" value={formatMoney(-101_200)} />
    </div>
  </Card>

  <Card title="Поля">
    <TextField id="dev-text" label="Наименование" bind:value={text} />
    <MoneyInput id="dev-money" label="Сумма" bind:value={amount} />
    <MoneyInput id="dev-money-lg" label="Крупная сумма" size="lg" bind:value={amount} />
    <MoneyInput id="dev-money-err" label="С ошибкой" value={null} error="Введите сумму" />
    <Select
      id="dev-select"
      label="Размер"
      options={[
        { value: 'a', label: 'А' },
        { value: 'b', label: 'Б' }
      ]}
      value={pick}
      onchange={(v: string) => {
        pick = v;
      }}
    />
    <Combobox
      id="dev-combo"
      label="Категория"
      options={categories}
      value={category}
      placeholder="Выберите"
      onchange={(v: string | null) => {
        category = v;
      }}
    />
    <Checkbox bind:checked>Чекбокс</Checkbox>
    <Switch bind:checked={on}>Переключатель</Switch>
  </Card>

  <Card title="Навигация">
    <Segmented
      label="Размер"
      options={[
        { value: 'sm', label: 'S' },
        { value: 'md', label: 'M' },
        { value: 'lg', label: 'L' }
      ]}
      value={size}
      onchange={(v: string) => {
        size = v;
      }}
    />
    <Tabs
      label="Вкладки"
      tabs={[
        { value: 'blocks', label: 'Блоки' },
        { value: 'table', label: 'Таблица' }
      ]}
      value={tab}
      onchange={(v: string) => {
        tab = v;
      }}
    />
  </Card>

  <Card title="Оверлеи и тосты">
    <div class="row">
      <Button
        onclick={() => {
          longModal = true;
        }}>Длинная модалка</Button
      >
      <Button
        onclick={() => {
          modal = true;
        }}>Модалка</Button
      >
      <Button
        onclick={() => {
          sheet = true;
        }}>Панель</Button
      >
      <Button
        onclick={() => {
          toasts.push({ message: 'Удалено', action: { label: 'Отменить', run: () => {} } });
        }}>Тост</Button
      >
      <Button
        onclick={() => {
          toasts.push({ message: 'Не удалось сохранить', kind: 'error', code: 'DB_BUSY' });
        }}>Ошибка</Button
      >
    </div>
  </Card>

  <Card title="Состояния">
    <Skeleton height="20px" />
    <Skeleton width="60%" />
    <EmptyState title="В сентябре пока нет трат" hint="Добавьте первую или импортируйте выписку" />
  </Card>
</div>

<div class="grid">
  <Card title="График: столбцы по статусам">
    <div class="plot">
      <Chart data={stackedData} spec={chartSpec({ type: 'stacked_bar', seriesBy: 'status' })} />
    </div>
  </Card>
  <Card title="График: норма сбережений">
    <div class="plot">
      <Chart data={rateData} spec={chartSpec({ type: 'line', metric: 'savings_rate' })} />
    </div>
  </Card>
  <Card title="График: структура по типам">
    <div class="plot">
      <Chart data={donutData} spec={chartSpec({ type: 'donut', groupBy: 'kind' })} />
    </div>
  </Card>
</div>

<Card title="Таблица, 5 000 строк">
  <div class="table">
    <DataTable
      {columns}
      {rows}
      rowKey={(r: Row) => r.id}
      label="Траты"
      {sort}
      onsort={toggleSort}
      {selected}
      onselect={toggleRow}
    >
      {#snippet cell(row: Row, column: Column)}
        {#if column.id === 'name'}
          {row.name}
        {:else if column.id === 'status'}
          <StatusChip status={row.status} />
        {:else}
          {formatMoney(row.amount)}
        {/if}
      {/snippet}
    </DataTable>
  </div>
</Card>

{#if modal}
  <Modal
    title="Модальное окно"
    onclose={() => {
      modal = false;
    }}
  >
    <p>Esc закрывает, Tab не выходит за пределы окна.</p>
    <Combobox
      id="dev-modal-combo"
      label="Категория в модалке"
      options={categories}
      value={null}
      onchange={() => {}}
    />
    <Button
      variant="primary"
      onclick={() => {
        modal = false;
      }}>Закрыть</Button
    >
  </Modal>
{/if}
{#if longModal}
  <Modal
    title="Модальное окно с длинным содержимым"
    onclose={() => {
      longModal = false;
    }}
  >
    {#each Array.from({ length: 40 }, (_, i) => i) as n (n)}
      <p>Строка {n + 1}: содержимое, которое не помещается в низкое окно.</p>
    {/each}
    {#snippet footer()}
      <Button
        variant="primary"
        onclick={() => {
          longModal = false;
        }}>Сохранить</Button
      >
    {/snippet}
  </Modal>
{/if}
{#if sheet}
  <Sheet
    title="Боковая панель"
    onclose={() => {
      sheet = false;
    }}><p>Содержимое панели.</p></Sheet
  >
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: var(--sp-4);
    margin-bottom: var(--sp-4);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-3);
  }
  .plot {
    height: 260px;
  }
  .table {
    display: flex;
    flex-direction: column;
    height: 420px;
  }
</style>

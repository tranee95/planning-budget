<script lang="ts">
  import ChartArea from '@lucide/svelte/icons/chart-area';
  import ChartBar from '@lucide/svelte/icons/chart-bar';
  import ChartColumn from '@lucide/svelte/icons/chart-column';
  import ChartColumnStacked from '@lucide/svelte/icons/chart-column-stacked';
  import ChartLine from '@lucide/svelte/icons/chart-line';
  import ChartPie from '@lucide/svelte/icons/chart-pie';
  import Hash from '@lucide/svelte/icons/hash';
  import Table from '@lucide/svelte/icons/table';
  import type { Component } from 'svelte';
  import type {
    ChartGroupByDto,
    ChartMetricDto,
    ChartSeriesByDto,
    ChartSortDto,
    ChartTypeDto
  } from '$lib/api/bindings';
  import {
    Button,
    Chart,
    EmptyState,
    Select,
    Sheet,
    Skeleton,
    Switch,
    TextField
  } from '$lib/components';
  import { currentMonth } from '$lib/format';
  import { categories } from '$lib/stores/categories.svelte';
  import {
    BOOL_OPTIONS,
    GROUPS,
    METRICS,
    PERIODS,
    SERIES,
    SORTS,
    TOP_N,
    TYPES,
    type PresetValue
  } from '$lib/charts/variants';
  import { defaultSize, getBuilderVm } from '../builder.svelte';
  import { getAnalyticsVm } from '../analytics.svelte';

  const builder = getBuilderVm();
  const analytics = getAnalyticsVm();

  const icons: Record<ChartTypeDto, Component<{ size?: number; 'aria-hidden'?: 'true' }>> = {
    bar: ChartColumn,
    stacked_bar: ChartColumnStacked,
    hbar: ChartBar,
    line: ChartLine,
    area: ChartArea,
    donut: ChartPie,
    table: Table,
    kpi: Hash
  };

  const draft = $derived(builder.draft);
  const period = $derived(draft.period);
  const preset = $derived<PresetValue>(period.preset ?? 'range');
  const range = $derived(period.preset === undefined ? { from: period.from, to: period.to } : null);
  const series = $derived(draft.seriesBy ?? 'none');
  const byMetric = $derived(draft.seriesBy === 'metric');
  const categoryColors = $derived(new Map(categories.items.map((c) => [c.id, c.color])));
  const creating = $derived(builder.mode.kind === 'create');
  const problem = $derived(builder.problem === null ? null : builder.reasonText(builder.problem));

  function setPreset(value: PresetValue): void {
    if (value === 'range') {
      const now = currentMonth();
      builder.change({ period: { from: `${now.slice(0, 4)}-01`, to: now } });
    } else {
      builder.change({ period: { preset: value } });
    }
  }

  function setRange(edge: 'from' | 'to', value: string): void {
    if (range === null || value === '') return;
    builder.change({ period: { ...range, [edge]: value } });
  }

  const reasonOf = (id: string): string | null => {
    const key = builder.blocked(id);
    return key === null ? null : builder.reasonText(key);
  };

  async function save(): Promise<void> {
    builder.saving = true;
    const spec = builder.draft;
    const ok =
      builder.mode.kind === 'edit'
        ? await analytics.updateChart(builder.mode.cardId, spec)
        : await analytics.addChart(spec, defaultSize(spec.type).w, defaultSize(spec.type).h);
    builder.saving = false;
    if (ok) builder.close();
  }
</script>

<Sheet
  title={creating ? 'Новый график' : 'Настройка графика'}
  width={380}
  onclose={() => {
    builder.close();
  }}
>
  <div class="form">
    <TextField
      id="builder-title"
      label="Название"
      bind:value={
        () => draft.title,
        (title: string) => {
          builder.change({ title });
        }
      }
    />

    <fieldset>
      <legend>Тип</legend>
      <div class="types" role="radiogroup" aria-label="Тип графика">
        {#each TYPES as type (type.value)}
          {@const reason = reasonOf(`type:${type.value}`)}
          {@const Icon = icons[type.value]}
          <button
            type="button"
            role="radio"
            aria-checked={draft.type === type.value}
            class:on={draft.type === type.value}
            disabled={reason !== null}
            title={reason ?? type.label}
            onclick={() => {
              builder.change({ type: type.value });
            }}
          >
            <Icon size={18} aria-hidden="true" />
            <span>{type.label}</span>
          </button>
        {/each}
      </div>
    </fieldset>

    {#if byMetric}
      <fieldset>
        <legend>Показатели (от 2 до 4)</legend>
        <div class="metrics">
          {#each METRICS as metric (metric.value)}
            {@const reason = reasonOf(`metric:${metric.value}`)}
            <label class:off={reason !== null} title={reason ?? undefined}>
              <input
                type="checkbox"
                checked={draft.metrics.includes(metric.value)}
                disabled={reason !== null && !draft.metrics.includes(metric.value)}
                onchange={() => {
                  builder.toggleMetric(metric.value);
                }}
              />
              {metric.label}
            </label>
          {/each}
        </div>
      </fieldset>
    {:else}
      <Select
        id="builder-metric"
        label="Показатель"
        options={METRICS}
        value={draft.metric}
        blocked={(v: ChartMetricDto) => reasonOf(`metric:${v}`)}
        onchange={(metric: ChartMetricDto) => {
          builder.change({ metric });
        }}
      />
    {/if}

    <Select
      id="builder-group"
      label="Разбить по"
      options={GROUPS}
      value={draft.groupBy}
      blocked={(v: ChartGroupByDto) => reasonOf(`group:${v}`)}
      onchange={(groupBy: ChartGroupByDto) => {
        builder.change({ groupBy });
      }}
    />

    <Select
      id="builder-series"
      label="Серии"
      options={SERIES}
      value={series}
      blocked={(v: ChartSeriesByDto | 'none') => reasonOf(`series:${v}`)}
      onchange={(next: ChartSeriesByDto | 'none') => {
        builder.setSeries(next);
      }}
    />

    <Select
      id="builder-period"
      label="Период"
      options={PERIODS}
      value={preset}
      onchange={setPreset}
    />
    {#if range !== null}
      <div class="range">
        <label
          >С<input
            type="month"
            value={range.from}
            onchange={(e) => {
              setRange('from', e.currentTarget.value);
            }}
          /></label
        >
        <label
          >По<input
            type="month"
            value={range.to}
            onchange={(e) => {
              setRange('to', e.currentTarget.value);
            }}
          /></label
        >
      </div>
    {/if}

    <TextField
      id="builder-filter"
      label="Фильтр"
      bind:value={
        () => draft.filter,
        (filter: string) => {
          builder.change({ filter });
        }
      }
      required={false}
      placeholder="тип:желания -кат:займы"
      hint="Язык поиска: кат:, тип:, статус:, сумма>, #тег, период:. Источник не поддерживается."
    />

    <fieldset>
      <legend>Опции</legend>
      <div class="options">
        {#each BOOL_OPTIONS as option (option.value)}
          {@const reason = reasonOf(`option:${option.value}`)}
          <span title={reason ?? undefined}>
            <Switch
              checked={draft.options[option.value]}
              disabled={reason !== null}
              onchange={() => {
                builder.toggle(option.value);
              }}>{option.label}</Switch
            >
          </span>
        {/each}
        <div class="pair">
          <Select
            id="builder-topn"
            label="Показать"
            options={TOP_N}
            value={String(draft.options.topN ?? 0)}
            onchange={(value) => {
              builder.changeOption({ topN: value === '0' ? null : Number(value) });
            }}
          />
          <Select
            id="builder-sort"
            label="Порядок"
            options={SORTS}
            value={draft.options.sort}
            onchange={(sort: ChartSortDto) => {
              builder.changeOption({ sort });
            }}
          />
        </div>
      </div>
    </fieldset>

    <section class="preview" aria-label="Предпросмотр" aria-live="polite">
      {#if builder.preview.status === 'ready'}
        {#if builder.preview.data.categories.length === 0}
          <EmptyState title="Нет данных за период" hint="Измените период или фильтр." />
        {:else}
          <Chart data={builder.preview.data} spec={draft} {categoryColors} />
        {/if}
      {:else if builder.preview.status === 'error'}
        <p class="problem" role="alert">{problem ?? builder.preview.message}</p>
      {:else}
        <Skeleton height="100%" />
      {/if}
    </section>
  </div>

  {#snippet footer()}
    <Button
      variant="ghost"
      onclick={() => {
        builder.close();
      }}>Отмена</Button
    >
    <Button variant="primary" disabled={!builder.canSave} loading={builder.saving} onclick={save}
      >{creating ? 'Добавить на панель' : 'Сохранить'}</Button
    >
  {/snippet}
</Sheet>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }
  fieldset {
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }
  legend {
    margin-bottom: var(--sp-1);
    padding: 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .types {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--sp-2);
  }
  .types button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-1);
    min-height: 56px;
    padding: var(--sp-2) var(--sp-1);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink-2);
    font: var(--fs-11) var(--font-sans);
    cursor: pointer;
  }
  .types button.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
  }
  .types button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .types button:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .metrics,
  .options {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    font-size: var(--fs-14);
  }
  .metrics label {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-height: var(--h-control-sm);
  }
  .metrics label.off {
    opacity: 0.45;
  }
  .range,
  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-3);
  }
  .range label {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .range input {
    height: var(--h-control);
    padding-inline: var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    background: var(--surface);
    color: var(--ink);
    font: var(--fs-14) var(--font-sans);
  }
  .preview {
    height: 240px;
    padding: var(--sp-3);
    border: 1px solid var(--line-2);
    border-radius: var(--r-md);
    background: var(--surface-2);
  }
  .problem {
    margin: 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

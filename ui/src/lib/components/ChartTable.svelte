<script lang="ts">
  import type { ChartDataDto } from '$lib/api/bindings';
  import { formatValue } from '$lib/charts/format';

  type Props = {
    data: ChartDataDto;
    caption: string;
    /** Таблица только для скринридеров: график на canvas недоступен без неё. */
    hidden?: boolean;
  };

  let { data, caption, hidden = false }: Props = $props();
</script>

<div class:hidden>
  <table>
    <caption>{caption}</caption>
    <thead>
      <tr>
        <th scope="col"></th>
        {#each data.series as s, index (index)}
          <th scope="col">{s.name}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each data.categories as category, i (i)}
        <tr>
          <th scope="row">{category}</th>
          {#each data.series as s, index (index)}
            {@const value = s.values[i]}
            <td>{value === null || value === undefined ? '—' : formatValue(value, data.unit)}</td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-13);
    font-variant-numeric: tabular-nums;
  }
  caption {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  th,
  td {
    padding: var(--sp-1) var(--sp-3);
    border-bottom: 1px solid var(--line-2);
    text-align: right;
    white-space: nowrap;
    color: var(--ink);
  }
  th[scope='row'],
  thead th:first-child {
    text-align: left;
  }
  thead th {
    color: var(--muted);
    font-weight: var(--fw-medium);
  }
  .hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .hidden table {
    width: max-content;
  }
</style>

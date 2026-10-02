<script lang="ts">
  type Row = { key: string; label: string; cells: readonly string[] };

  type Props = {
    /** Подпись таблицы для скринридеров. */
    caption: string;
    /** Заголовки колонок, первым — заголовок колонки строк. */
    headers: readonly string[];
    rows: readonly Row[];
  };

  let { caption, headers, rows }: Props = $props();
</script>

<div class="scroll">
  <table>
    <caption>{caption}</caption>
    <thead>
      <tr>
        {#each headers as header, index (index)}
          <th scope="col">{header}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.key)}
        <tr>
          <th scope="row">{row.label}</th>
          {#each row.cells as cell, index (index)}
            <td>{cell}</td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .scroll {
    overflow-x: auto;
  }
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
  }
  th[scope='row'],
  thead th:first-child {
    text-align: left;
  }
  thead th {
    color: var(--muted);
    font-weight: var(--fw-medium);
  }
</style>

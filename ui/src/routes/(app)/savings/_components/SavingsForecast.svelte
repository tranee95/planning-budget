<script lang="ts">
  import { Card, Chart, SimpleTable } from '$lib/components';
  import { currentMonth, formatMoney } from '$lib/format';
  import { getSavingsVm } from '../savings.svelte';

  const vm = getSavingsVm();
  const item = $derived(vm.selected);
  const past = $derived(vm.data !== null && vm.data.year < Number(currentMonth().slice(0, 4)));
</script>

<Card title="Прогноз на 5 лет">
  {#if item !== null && vm.data !== null}
    <p class="from">
      От баланса на конец {vm.data.year} года: <strong>{formatMoney(item.balance)}</strong>
    </p>
    <div class="plot">
      <Chart data={item.forecastChart} type="line" title="Прогноз баланса накопления на 5 лет" />
    </div>
    {#if past}
      <p class="note">
        Для прошлого года это расчёт «что было бы»: от баланса на конец {vm.data.year} года с взносами
        по его средним и сегодняшними лимитами. Месяцы прогноза могли уже пройти.
      </p>
    {/if}
    <SimpleTable
      caption="Итоги прогноза по сценариям"
      headers={[
        'Сценарий',
        'Взнос в месяц',
        'Через 1 год',
        'Через 3 года',
        'Через 5 лет',
        'Из них купоны'
      ]}
      rows={item.scenarios.map((s) => ({
        key: s.scenario,
        label: s.name,
        cells: [
          formatMoney(s.contribution),
          formatMoney(s.y1),
          formatMoney(s.y3),
          formatMoney(s.y5),
          formatMoney(s.coupons60)
        ]
      }))}
    />
    <p class="disclaimer">
      Ставка и выплаты не гарантированы; расчёт при постоянной ставке и реинвестировании купонов.
      Прогноз выделен пунктиром и не является фактом.
    </p>
  {/if}
</Card>

<style>
  .from {
    margin: 0;
    color: var(--ink-2);
    font-size: var(--fs-14);
  }
  .plot {
    height: 280px;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .disclaimer {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-12);
  }
</style>

<script lang="ts">
  import { deviationTone } from '$lib/deviation';
  import type { MonthPlanDto } from '$lib/api/bindings';
  import { Button, Card, FormulaHint, Kpi, StatusDot } from '$lib/components';
  import { formatMoney, formatMonthGenitive, formatMonthLower, shiftMonth } from '$lib/format';
  import { FORMULAS } from '$lib/formulas';
  import type { PlanLine } from '../overview.svelte';

  type Props = {
    plan: MonthPlanDto;
    lines: readonly PlanLine[];
    busy: boolean;
    /** «Скопировать план» предлагается, пока в месяце нет плановых трат. */
    canCopy: boolean;
    onlock: () => void;
    onunlock: () => void;
    oncopy: () => void;
    /** Открыть мастер: заполнить месяц с нуля. */
    onstart: () => void;
    /** Месяц плана `ГГГГ-ММ`: из него строятся подписи пустого состояния. */
    month: string;
    /** Отметить погашение долга оплаченным или вернуть в план. */
    onrepay: (paymentId: number, paid: boolean) => void;
  };

  let { plan, lines, busy, canCopy, onlock, onunlock, oncopy, onstart, month, onrepay }: Props =
    $props();

  const nothingYet = $derived(plan.balance === 'empty' && !plan.locked);
  /** «октябрь» для «Составьте план на октябрь» (винительный совпадает с именительным). */
  const monthName = $derived(formatMonthLower(month).split(' ')[0] ?? '');
  /** «сентября» для «Скопировать план из сентября». */
  const previousName = $derived(formatMonthGenitive(shiftMonth(month, -1)).split(' ')[0] ?? '');

  const balanceHint = $derived(
    plan.balance === 'empty'
      ? 'внесите доход и распределите его по статьям'
      : plan.balance === 'balanced'
        ? 'каждый рубль дохода получил назначение'
        : plan.balance === 'unallocated'
          ? 'доход без назначения'
          : `план превышает доход на ${formatMoney(Math.abs(plan.unallocated))}`
  );
  const signed = (kopecks: number): string =>
    kopecks > 0 ? `+${formatMoney(kopecks)}` : formatMoney(kopecks);
</script>

<Card title="План месяца">
  {#snippet actions()}
    {#if plan.locked}
      <span class="locked">План зафиксирован</span>
      <Button variant="ghost" size="sm" loading={busy} onclick={onunlock}>Разблокировать</Button>
    {:else}
      {#if canCopy && !nothingYet}
        <Button variant="secondary" size="sm" loading={busy} onclick={oncopy}>
          Скопировать план из прошлого месяца
        </Button>
      {/if}
      {#if !nothingYet}
        <Button variant="primary" size="sm" loading={busy} onclick={onlock}
          >Зафиксировать план</Button
        >
      {/if}
    {/if}
  {/snippet}

  {#if nothingYet}
    <div class="start">
      <h3>Составьте план на {monthName}</h3>
      <p>
        Внесите доход и распределите его по статьям: «Не распределено» должно дойти до нуля, после
        этого план можно зафиксировать.
      </p>
      <div class="start-actions">
        {#if canCopy}
          <Button variant="primary" loading={busy} onclick={oncopy}
            >Скопировать план из {previousName}</Button
          >
        {/if}
        <Button variant={canCopy ? 'secondary' : 'primary'} onclick={onstart}>Начать с нуля</Button>
      </div>
    </div>
  {:else}
    <div class="totals">
      <Kpi label="Доход" amount={plan.income} formula={FORMULAS.planIncome} />
      <Kpi label="План трат" amount={plan.planExpenses} formula={FORMULAS.planExpenses} />
      <Kpi label="План сбережений" amount={plan.planSavings} formula={FORMULAS.planSavings} />
      <Kpi label="План погашений" amount={plan.planRepayments} formula={FORMULAS.planRepayments} />
      <div class="unallocated state-{plan.balance}" aria-live="polite">
        <span class="label">
          Не распределено
          <FormulaHint label="Не распределено" lines={FORMULAS.unallocated.lines} />
        </span>
        <span class="value num">{formatMoney(plan.unallocated)}</span>
        <span class="hint">{balanceHint}</span>
      </div>
    </div>

    <div class="detail">
      <table>
        <caption>План и факт по статьям</caption>
        <thead>
          <tr>
            <th scope="col">Статья</th>
            <th scope="col">План</th>
            <th scope="col">Фактически</th>
            <th
              scope="col"
              title="Оплачено и в долг плюс то, что ещё запланировано: прогноз расхода за месяц"
            >
              Факт + план
            </th>
            <th scope="col">
              Отклонение
              <FormulaHint label={FORMULAS.planFact.label} lines={FORMULAS.planFact.lines} />
            </th>
          </tr>
        </thead>
        <tbody>
          {#each lines as { category, row } (category.id)}
            <tr>
              <th scope="row">{category.name}</th>
              <td class="num">{formatMoney(row.plan)}</td>
              <td class="num">{formatMoney(row.actual)}</td>
              <td class="num">{formatMoney(row.fact)}</td>
              <td
                class="num dev"
                class:over={deviationTone(category.kind, row.deviation) === 'bad'}
                class:good={deviationTone(category.kind, row.deviation) === 'good'}
                >{signed(row.deviation)}</td
              >
            </tr>
          {:else}
            <tr><td class="empty" colspan="5">План пока пуст.</td></tr>
          {/each}
        </tbody>
      </table>

      <div class="side">
        {#if plan.repayments.length > 0}
          <section class="repayments" aria-labelledby="repayments-title">
            <h3 id="repayments-title">Погашения долгов</h3>
            <ul>
              {#each plan.repayments as r (r.paymentId)}
                <li>
                  <span class="who">{r.lender}</span>
                  <span class="num">{formatMoney(r.amount)}</span>
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={busy}
                    onclick={() => {
                      onrepay(r.paymentId, r.status !== 'paid');
                    }}
                  >
                    {r.status === 'paid' ? 'Оплачено · вернуть' : 'Отметить оплаченным'}
                  </Button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        <dl class="summary">
          <div>
            <dt><StatusDot status="unplanned" />Незапланировано</dt>
            <dd class="num">{formatMoney(plan.unplanned)}</dd>
          </div>
          <div>
            <dt><StatusDot status="debt" />Взято в долг</dt>
            <dd class="num">{formatMoney(plan.borrowed)}</dd>
          </div>
          <div>
            <dt>
              Отложено
              <FormulaHint label={FORMULAS.planSaved.label} lines={FORMULAS.planSaved.lines} />
            </dt>
            <dd class="num">
              {formatMoney(plan.saved)}
              <span class="of">из плана {formatMoney(plan.planSavings)}</span>
            </dd>
          </div>
        </dl>
      </div>
    </div>
  {/if}
</Card>

<style>
  /* Пустой месяц: одно содержательное сообщение вместо девяти нулей */
  .start {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-3);
    padding-block: var(--sp-4);
  }
  .start h3 {
    margin: 0;
    font-size: var(--fs-17);
    font-weight: var(--fw-semibold);
  }
  .start p {
    max-width: 56ch;
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-13);
    line-height: var(--lh-body);
  }
  .start-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }
  .locked {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 var(--sp-2);
    border-radius: var(--r-full);
    background: var(--paid-bg);
    color: var(--paid);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .totals {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--sp-4);
    align-items: start;
  }
  .unallocated {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    padding: var(--sp-3) var(--sp-4);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    min-width: 0;
  }
  .unallocated .label {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    font-size: var(--fs-12);
  }
  .unallocated .value {
    font-size: var(--fs-26);
    font-weight: var(--fw-semibold);
    line-height: var(--lh-tight);
    font-variant-numeric: proportional-nums;
  }
  .unallocated .hint {
    font-size: var(--fs-12);
  }
  .unallocated.state-empty {
    background: var(--surface-2);
    color: var(--muted);
  }
  .unallocated.state-balanced {
    background: var(--paid-bg);
    border-color: var(--paid-bar);
    color: var(--paid);
  }
  .unallocated.state-unallocated {
    background: var(--debt-bg);
    border-color: var(--debt-bar);
    color: var(--debt);
  }
  .unallocated.state-over {
    background: var(--unpl-bg);
    border-color: var(--unpl-bar);
    color: var(--unpl);
  }
  .detail {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr);
    gap: var(--sp-6);
    /* таблица и правая колонка начинаются с одной линии */
    align-items: start;
  }
  .locked {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding-inline: var(--sp-3);
    border-radius: var(--r-full);
    background: var(--success-subtle);
    color: var(--success);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  @container (max-width: 880px) {
    .detail {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  table {
    width: 100%;
    /* ширина колонок не зависит от содержимого: заголовки и числа стоят на одних местах и в пустом, и в полном плане */
    table-layout: fixed;
    border-collapse: collapse;
    font-size: var(--fs-13);
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
    padding: var(--sp-2);
    border-bottom: 1px solid var(--line-2);
    text-align: right;
  }
  thead th {
    color: var(--muted);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  th:first-child {
    width: 28%;
    text-align: left;
    font-weight: var(--fw-medium);
  }
  .dev {
    color: var(--muted);
  }
  .dev.good {
    color: var(--paid);
    font-weight: var(--fw-semibold);
  }
  .dev.over {
    color: var(--unpl);
    font-weight: var(--fw-semibold);
  }
  .empty {
    color: var(--muted);
    text-align: left;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    min-width: 0;
  }
  .repayments h3 {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-13);
    font-weight: var(--fw-semibold);
  }
  .repayments ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-13);
  }
  .repayments li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) 0;
    border-bottom: 1px solid var(--line-2);
  }
  .summary {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    margin: 0;
    font-size: var(--fs-13);
  }
  .summary div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-2);
  }
  dt {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
  }
  dd {
    margin: 0;
    font-weight: var(--fw-semibold);
  }
  .of {
    color: var(--muted);
    font-weight: var(--fw-regular);
  }
</style>

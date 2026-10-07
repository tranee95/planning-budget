<script lang="ts">
  import type { MonthPlanDto } from '$lib/api/bindings';
  import { Button, Card, Kpi, StatusDot } from '$lib/components';
  import { formatMoney } from '$lib/format';
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
    /** Отметить погашение долга оплаченным или вернуть в план. */
    onrepay: (paymentId: number, paid: boolean) => void;
  };

  let { plan, lines, busy, canCopy, onlock, onunlock, oncopy, onrepay }: Props = $props();

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
      <span class="locked">План готов</span>
      <Button variant="secondary" size="sm" loading={busy} onclick={onunlock}>Разблокировать</Button
      >
    {:else}
      {#if canCopy}
        <Button variant="secondary" size="sm" loading={busy} onclick={oncopy}>
          Скопировать план из прошлого месяца
        </Button>
      {/if}
      <Button variant="primary" size="sm" loading={busy} onclick={onlock}>План готов</Button>
    {/if}
  {/snippet}

  <div class="totals">
    <Kpi label="Доход" amount={plan.income} />
    <Kpi label="План трат" amount={plan.planExpenses} />
    <Kpi label="План сбережений" amount={plan.planSavings} />
    <Kpi label="План погашений" amount={plan.planRepayments} />
    <div class="unallocated {plan.balance}" aria-live="polite">
      <span class="label">Не распределено</span>
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
          <th scope="col">Потрачено</th>
          <th scope="col">Отклонение</th>
        </tr>
      </thead>
      <tbody>
        {#each lines as { category, row } (category.id)}
          <tr>
            <th scope="row">{category.name}</th>
            <td class="num">{formatMoney(row.plan)}</td>
            <td class="num">{formatMoney(row.fact)}</td>
            <td class="num dev" class:over={row.deviation > 0}>{signed(row.deviation)}</td>
          </tr>
        {:else}
          <tr><td class="empty" colspan="4">План пока пуст.</td></tr>
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
          <dt>Отложено</dt>
          <dd class="num">
            {formatMoney(plan.saved)} <span class="of">/ {formatMoney(plan.planSavings)}</span>
          </dd>
        </div>
      </dl>
    </div>
  </div>
</Card>

<style>
  .locked {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 var(--sp-2);
    border-radius: 999px;
    background: var(--paid-bg);
    color: var(--paid);
    font-size: var(--fs-12);
    font-weight: var(--fw-medium);
  }
  .totals {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--sp-4);
    align-items: stretch;
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
    font-size: var(--fs-12);
  }
  .unallocated .value {
    font-size: var(--fs-26);
    font-weight: var(--fw-semibold);
    line-height: var(--lh-tight);
    font-variant-numeric: tabular-nums;
  }
  .unallocated .hint {
    font-size: var(--fs-12);
  }
  .unallocated.empty {
    background: var(--surface-2);
    color: var(--muted);
  }
  .unallocated.balanced {
    background: var(--paid-bg);
    border-color: var(--paid-bar);
    color: var(--paid);
  }
  .unallocated.unallocated {
    background: var(--debt-bg);
    border-color: var(--debt-bar);
    color: var(--debt);
  }
  .unallocated.over {
    background: var(--unpl-bg);
    border-color: var(--unpl-bar);
    color: var(--unpl);
  }
  .detail {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr);
    gap: var(--sp-6);
  }
  @container (max-width: 880px) {
    .detail {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  table {
    width: 100%;
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
    text-align: left;
    font-weight: var(--fw-medium);
  }
  .dev {
    color: var(--muted);
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

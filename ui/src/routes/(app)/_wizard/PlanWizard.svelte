<script lang="ts">
  import { untrack } from 'svelte';
  import { Button, MoneyInput, Modal, Segmented, Skeleton, TextField } from '$lib/components';
  import { formatMoney } from '$lib/format';
  import { onboarding } from '$lib/stores/onboarding.svelte';
  import { WIZARD_STEPS, WizardVm } from './wizard.svelte';

  const vm = new WizardVm();
  void vm.init();

  // «Не распределено» пересчитывает Rust, когда меняется ввод.
  $effect(() => {
    if (vm.inputKey) {
      untrack(() => {
        void vm.refreshPreview();
      });
    }
  });

  const KINDS = [
    { value: 'percent', label: '% от дохода' },
    { value: 'fixed', label: 'Сумма' }
  ] as const;

  const preview = $derived(vm.preview);
  const title = $derived(`План на ${vm.monthName}: ${WIZARD_STEPS[vm.step] ?? ''}`);
</script>

<Modal
  {title}
  width={760}
  onclose={() => {
    onboarding.closeWizard();
  }}
>
  <p class="steps" aria-label="Шаг {vm.step + 1} из {WIZARD_STEPS.length}">
    {#each WIZARD_STEPS as name, index (name)}
      <span class="dot" class:on={index === vm.step} class:done={index < vm.step}>{name}</span>
    {/each}
  </p>

  {#if vm.loading}
    <Skeleton height="240px" />
  {:else if vm.step === 0}
    <p class="lead">
      Внесите ожидаемые поступления месяца. По мере получения вы отметите их как полученные.
    </p>
    <div class="rows">
      {#each vm.incomes as row, index (index)}
        <div class="row income">
          <TextField id="wiz-income-name-{index}" label="Источник" bind:value={row.name} />
          <MoneyInput id="wiz-income-amount-{index}" label="Сумма" bind:value={row.amount} />
          {#if vm.incomes.length > 1}
            <Button
              variant="ghost"
              size="sm"
              onclick={() => {
                vm.removeIncome(index);
              }}>Убрать</Button
            >
          {/if}
        </div>
      {/each}
    </div>
    <div>
      <Button
        variant="secondary"
        size="sm"
        onclick={() => {
          vm.addIncome();
        }}>+ Ещё поступление</Button
      >
    </div>
  {:else if vm.step === 1}
    <p class="lead">
      Сколько вы планируете потратить по каждой статье. Пустые статьи в план не попадут.
    </p>
    <div class="grid">
      {#each vm.lines as row (row.category.id)}
        <MoneyInput
          id="wiz-line-{row.category.id}"
          label={row.category.name}
          bind:value={row.amount}
        />
      {/each}
    </div>
  {:else if vm.step === 2}
    <p class="lead">
      Сколько откладывать на каждое накопление: долей дохода или фиксированной суммой.
    </p>
    {#if vm.savings.length === 0}
      <p class="muted">Накоплений нет: добавьте категорию типа «Сбережения» в «Категориях».</p>
    {/if}
    <div class="rows">
      {#each vm.savings as row (row.category.id)}
        <div class="row saving">
          <span class="name">{row.category.name}</span>
          <Segmented
            label="План: {row.category.name}"
            options={KINDS}
            value={row.kind}
            onchange={(value: 'percent' | 'fixed') => {
              row.kind = value;
            }}
          />
          {#if row.kind === 'percent'}
            <TextField
              id="wiz-saving-percent-{row.category.id}"
              label="Доля дохода, %"
              bind:value={row.percent}
            />
          {:else}
            <MoneyInput
              id="wiz-saving-fixed-{row.category.id}"
              label="Сумма в месяц"
              bind:value={row.fixed}
            />
          {/if}
        </div>
      {/each}
    </div>
  {:else if preview}
    <div class="result {preview.balance}" aria-live="polite">
      <span class="label">Не распределено</span>
      <span class="value num">{formatMoney(preview.unallocated)}</span>
      <span class="hint">
        {#if preview.balance === 'balanced'}
          каждый рубль дохода получил назначение
        {:else if preview.balance === 'unallocated'}
          доход без назначения: вернитесь и распределите остаток или оставьте как есть
        {:else if preview.balance === 'over'}
          план превышает доход на {formatMoney(Math.abs(preview.unallocated))}
        {:else}
          внесите доход и суммы по статьям
        {/if}
      </span>
    </div>
    <dl class="totals">
      <div>
        <dt>Доход</dt>
        <dd class="num">{formatMoney(preview.income)}</dd>
      </div>
      <div>
        <dt>План трат</dt>
        <dd class="num">{formatMoney(preview.planExpenses)}</dd>
      </div>
      <div>
        <dt>План сбережений</dt>
        <dd class="num">{formatMoney(preview.planSavings)}</dd>
      </div>
    </dl>
    <ul class="lines">
      {#each vm.planLines as line (line.name)}
        <li><span>{line.name}</span><span class="num">{formatMoney(line.plan)}</span></li>
      {/each}
    </ul>
    <p class="muted">
      «План готов» запоминает эти суммы: в течение месяца вы будете сравнивать с ними факт.
    </p>
  {/if}

  {#if vm.error}<p class="error" role="alert">{vm.error}</p>{/if}

  {#snippet footer()}
    {#if preview && vm.step < WIZARD_STEPS.length - 1}
      <span class="mini"
        >Не распределено: <b class="num">{formatMoney(preview.unallocated)}</b></span
      >
    {/if}
    <Button
      variant="ghost"
      onclick={() => {
        onboarding.closeWizard();
      }}>Позже</Button
    >
    {#if vm.step > 0}
      <Button
        variant="secondary"
        onclick={() => {
          vm.back();
        }}>Назад</Button
      >
    {/if}
    {#if vm.last}
      <Button variant="primary" loading={vm.applying} onclick={() => void vm.apply()}
        >План готов</Button
      >
    {:else}
      <Button
        variant="primary"
        onclick={() => {
          vm.next();
        }}>Далее</Button
      >
    {/if}
  {/snippet}
</Modal>

<style>
  .steps {
    display: flex;
    gap: var(--sp-2);
    margin: 0 0 var(--sp-4);
    padding: 0;
    flex-wrap: wrap;
  }
  .dot {
    padding: 2px var(--sp-3);
    border-radius: var(--r-full);
    background: var(--line-2);
    color: var(--muted);
    font-size: var(--fs-12);
  }
  .dot.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: var(--fw-semibold);
  }
  .dot.done {
    color: var(--ink-2);
  }
  .lead,
  .muted {
    margin: 0 0 var(--sp-3);
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    margin-bottom: var(--sp-3);
  }
  .row {
    display: grid;
    align-items: end;
    gap: var(--sp-3);
  }
  .row.income {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
  }
  .row.saving {
    grid-template-columns: 140px auto minmax(0, 1fr);
  }
  .name {
    font-weight: var(--fw-medium);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--sp-3) var(--sp-4);
  }
  .result {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    padding: var(--sp-5);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    margin-bottom: var(--sp-4);
  }
  .result .value {
    font-size: var(--fs-26);
    font-weight: var(--fw-semibold);
    line-height: var(--lh-tight);
  }
  .result .hint,
  .result .label {
    font-size: var(--fs-13);
  }
  .result.empty {
    background: var(--surface-2);
    color: var(--muted);
  }
  .result.balanced {
    background: var(--paid-bg);
    border-color: var(--paid-bar);
    color: var(--paid);
  }
  .result.unallocated {
    background: var(--debt-bg);
    border-color: var(--debt-bar);
    color: var(--debt);
  }
  .result.over {
    background: var(--unpl-bg);
    border-color: var(--unpl-bar);
    color: var(--unpl);
  }
  .totals {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sp-3);
    margin: 0 0 var(--sp-3);
    font-size: var(--fs-13);
  }
  .totals div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .totals dt {
    color: var(--muted);
  }
  .totals dd {
    margin: 0;
    font-weight: var(--fw-semibold);
  }
  .lines {
    display: flex;
    flex-direction: column;
    margin: 0 0 var(--sp-3);
    padding: 0;
    list-style: none;
    font-size: var(--fs-13);
  }
  .lines li {
    display: flex;
    justify-content: space-between;
    padding: var(--sp-1) 0;
    border-bottom: 1px solid var(--line-2);
  }
  .mini {
    margin-inline-end: auto;
    color: var(--muted);
    font-size: var(--fs-13);
  }
  .mini b {
    color: var(--ink);
  }
  .error {
    margin: var(--sp-3) 0 0;
    color: var(--unpl);
    font-size: var(--fs-13);
  }
</style>

import type {
  AppError,
  DebtDto,
  DebtInput,
  DebtPatchDto,
  DebtPaymentDto,
  DebtPaymentStatusDto,
  DebtScheduleKindDto,
  DebtsOverviewDto,
  PlanRepaymentDto,
  SchedulePaymentDto,
  TransactionDto
} from '../bindings';
import { shiftMonth } from '$lib/format';

/**
 * Мок долгов: только заполняет поля DTO. Формулы остатка и графика живут в Rust,
 * поэтому здесь они упрощены.
 */
let debts: DebtDto[] = [];
let deleted = new Map<number, DebtDto>();
let nextDebtId = 1;
let nextPaymentId = 1000;

export function resetMockDebts(): void {
  debts = [];
  deleted = new Map();
  nextDebtId = 1;
  nextPaymentId = 1000;
}

function reject(error: AppError): Promise<never> {
  // IPC отдаёт AppError значением, а не Error
  // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
  return Promise.reject(error);
}

const scheduleError = (): Promise<never> =>
  reject({ code: 'Validation', messageKey: 'errors.debt.schedule', field: null });

/** Остаток, доля и ближайший платёж из строк графика. */
function refresh(debt: DebtDto): DebtDto {
  const paid = debt.payments.filter((p) => p.status === 'paid').reduce((s, p) => s + p.amount, 0);
  const next = debt.payments.find((p) => p.status === 'planned') ?? null;
  return {
    ...debt,
    remaining: debt.amount - paid,
    paidBp: Math.round((paid / debt.amount) * 10_000),
    closed: debt.amount - paid === 0,
    nextPayment: next
  };
}

function rowsOf(schedule: readonly SchedulePaymentDto[]): DebtPaymentDto[] {
  return schedule.map((s) => ({
    id: nextPaymentId++,
    month: s.month,
    amount: s.amount,
    status: 'planned',
    paidDate: null
  }));
}

function validSchedule(
  amount: number,
  taken: string,
  rows: readonly SchedulePaymentDto[]
): boolean {
  const months = new Set(rows.map((r) => r.month));
  return (
    rows.length > 0 &&
    months.size === rows.length &&
    rows.every((r) => r.amount > 0 && r.month >= taken) &&
    rows.reduce((s, r) => s + r.amount, 0) === amount
  );
}

function preview(amount: number, taken: string, kind: DebtScheduleKindDto): SchedulePaymentDto[] {
  if (kind.kind === 'single') {
    if (kind.month < taken || amount <= 0) throw new Error('schedule');
    return [{ month: kind.month, amount }];
  }
  if (kind.months < 1 || kind.months > 600 || amount < kind.months) throw new Error('schedule');
  const part = Math.floor(amount / kind.months);
  return Array.from({ length: kind.months }, (_, i) => ({
    month: shiftMonth(taken, i + 1),
    amount: i === kind.months - 1 ? amount - part * (kind.months - 1) : part
  }));
}

/** Погашения месяца для плана (мок `plan_month`). */
export function repaymentsOfMonth(month: string): PlanRepaymentDto[] {
  return debts.flatMap((d) =>
    d.payments
      .filter((p) => p.month === month)
      .map((p) => ({
        paymentId: p.id,
        debtId: d.id,
        lender: d.lender,
        amount: p.amount,
        status: p.status
      }))
  );
}

function summaryFor(month: string, includeClosed: boolean): DebtsOverviewDto {
  const open = debts.filter((d) => !d.closed);
  const monthPayments = debts.flatMap((d) => d.payments).filter((p) => p.month === month);
  return {
    debts: debts.filter((d) => includeClosed || !d.closed),
    openCount: open.length,
    closedCount: debts.length - open.length,
    remainingTotal: open.reduce((s, d) => s + d.remaining, 0),
    paymentsPlanned: monthPayments.reduce((s, p) => s + p.amount, 0),
    paymentsPaid: monthPayments.filter((p) => p.status === 'paid').reduce((s, p) => s + p.amount, 0)
  };
}

function save(debt: DebtDto): DebtDto {
  const next = refresh(debt);
  debts = debts.some((d) => d.id === next.id)
    ? debts.map((d) => (d.id === next.id ? next : d))
    : [...debts, next];
  return next;
}

export function createDebtHandlers(findTransaction: (id: number) => TransactionDto | undefined) {
  const build = (
    input: Omit<DebtInput, 'schedule'> & { transactionId?: number },
    schedule: readonly SchedulePaymentDto[]
  ): DebtDto | null => {
    if (!validSchedule(input.amount, input.takenMonth, schedule)) return null;
    return refresh({
      id: nextDebtId++,
      lender: input.lender.trim(),
      amount: input.amount,
      takenMonth: input.takenMonth,
      takenDate: input.takenDate,
      categoryId: input.categoryId,
      transactionId: input.transactionId ?? null,
      comment: input.comment,
      closed: false,
      remaining: input.amount,
      paidBp: 0,
      nextPayment: null,
      payments: rowsOf(schedule)
    });
  };
  return {
    debts_list: (args: { month: string; includeClosed: boolean }) =>
      summaryFor(args.month, args.includeClosed),
    debts_create: (args: { input: DebtInput }) => {
      if (args.input.lender.trim() === '') {
        return reject({ code: 'Validation', messageKey: 'errors.debt.lender_empty', field: null });
      }
      const debt = build(args.input, args.input.schedule);
      return debt ? save(debt) : scheduleError();
    },
    debt_from_tx: (args: { txId: number; lender: string; schedule: SchedulePaymentDto[] }) => {
      const tx = findTransaction(args.txId);
      if (!tx) return reject({ code: 'NotFound', entity: 'record', id: args.txId });
      const debt = build(
        {
          lender: args.lender,
          amount: tx.amount,
          takenMonth: tx.month,
          takenDate: tx.date,
          categoryId: tx.categoryId,
          comment: null,
          transactionId: tx.id
        },
        args.schedule
      );
      return debt ? save(debt) : scheduleError();
    },
    debts_update: (args: { id: number; patch: DebtPatchDto }) => {
      const current = debts.find((d) => d.id === args.id);
      if (!current) return reject({ code: 'NotFound', entity: 'record', id: args.id });
      const { patch } = args;
      const amount = patch.amount ?? current.amount;
      const newSchedule = patch.schedule ?? undefined;
      const paid = current.payments.filter((p) => p.status === 'paid');
      let payments = current.payments;
      if (newSchedule !== undefined || amount !== current.amount) {
        if (newSchedule === undefined) return scheduleError();
        const full = [...paid, ...newSchedule];
        if (!validSchedule(amount, current.takenMonth, full)) return scheduleError();
        payments = [...paid, ...rowsOf(newSchedule)].sort((a, b) => a.month.localeCompare(b.month));
      }
      return save({
        ...current,
        lender: patch.lender?.trim() ?? current.lender,
        amount,
        categoryId: patch.categoryId === undefined ? current.categoryId : patch.categoryId,
        comment: patch.comment === undefined ? current.comment : patch.comment,
        payments
      });
    },
    debts_delete: (args: { id: number }) => {
      const current = debts.find((d) => d.id === args.id);
      if (!current) return reject({ code: 'NotFound', entity: 'record', id: args.id });
      debts = debts.filter((d) => d.id !== args.id);
      deleted.set(current.id, current);
      return current;
    },
    debts_restore: (args: { id: number }) => {
      const row = deleted.get(args.id);
      if (!row) return reject({ code: 'NotFound', entity: 'record', id: args.id });
      deleted.delete(args.id);
      return save(row);
    },
    debt_payment_set_status: (args: {
      paymentId: number;
      status: DebtPaymentStatusDto;
      paidDate: string | null;
    }) => {
      const debt = debts.find((d) => d.payments.some((p) => p.id === args.paymentId));
      if (!debt) return reject({ code: 'NotFound', entity: 'record', id: args.paymentId });
      return save({
        ...debt,
        payments: debt.payments.map((p) =>
          p.id === args.paymentId
            ? { ...p, status: args.status, paidDate: args.status === 'paid' ? args.paidDate : null }
            : p
        )
      });
    },
    debt_schedule_preview: (args: {
      amount: number;
      takenMonth: string;
      kind: DebtScheduleKindDto;
    }) => {
      try {
        return preview(args.amount, args.takenMonth, args.kind);
      } catch {
        return scheduleError();
      }
    }
  };
}

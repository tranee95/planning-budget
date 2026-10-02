import type { BondsDto, ForecastScenarioDto } from '../bindings';
import { mockSettings } from './budget';

/**
 * Мок `bonds_projection`: те же формулы, что в `core::calc`, на постоянном взносе
 * 30 000 ₽ в месяц с января по сентябрь 2026. Настоящий расчёт живёт в Rust.
 */

const MONTHLY_SAVINGS = 3_000_000;
const MONTHS_WITH_DATA = 9;

const pad = (n: number): string => String(n).padStart(2, '0');

function scenario(
  kind: 'a' | 'b' | 'c',
  contribution: number,
  start: number,
  monthly: number
): ForecastScenarioDto {
  let balance = start;
  const balances: number[] = [];
  for (let n = 1; n <= 60; n++) {
    balance = balance * (1 + monthly) + contribution;
    balances.push(Math.round(balance));
  }
  return {
    scenario: kind,
    contribution,
    y1: balances[11] ?? 0,
    y3: balances[35] ?? 0,
    y5: balances[59] ?? 0,
    coupons60: Math.round(balance - start - contribution * 60),
    balances
  };
}

export function mockBonds(year: number): BondsDto {
  const settings = mockSettings();
  const effective = (settings.bondsRateBp / 10_000) * (1 - settings.bondsCouponTaxBp / 10_000);
  const monthly = (1 + effective) ** (1 / 12) - 1;
  const startMonth = settings.bondsInitialMonth;
  let balance = settings.bondsInitialBalance;
  let deposited = settings.bondsInitialBalance;
  const months: BondsDto['months'] = [];
  // накопление идёт от стартового месяца; в список попадают месяцы запрошенного года
  for (let m = 1; m <= 12; m++) {
    const key = `${String(year)}-${pad(m)}`;
    if (key < startMonth) {
      months.push({ month: key, savings: 0, coupon: 0, balance: 0, deposited: 0, couponIncome: 0 });
      continue;
    }
    const savings = year === 2026 && m <= MONTHS_WITH_DATA ? MONTHLY_SAVINGS : 0;
    const coupon = balance * monthly;
    balance += savings + coupon;
    deposited += savings;
    months.push({
      month: `${String(year)}-${pad(m)}`,
      savings,
      coupon: Math.round(coupon),
      balance: Math.round(balance),
      deposited,
      couponIncome: Math.round(balance - deposited)
    });
  }
  const start = Math.round(balance);
  return {
    year,
    effectiveRate: effective,
    monthlyRate: monthly,
    months,
    decBalance: start,
    scenarios: [
      scenario('a', 3_486_857, start, monthly),
      scenario('b', 5_893_113, start, monthly),
      scenario('c', MONTHLY_SAVINGS, start, monthly)
    ]
  };
}

import { expect, it } from 'vitest';
import { isReadCommand } from './install';

it('чтение не сбрасывает кеш расчётов, запись сбрасывает', () => {
  for (const read of [
    'summary_month',
    'summary_year',
    'summary_series',
    'plan_month',
    'plan_preview',
    'tx_list',
    'tx_search',
    'tx_suggest',
    'tx_category_usage',
    'categories_list',
    'categories_defaults',
    'limits_history',
    'analytics_run',
    'analytics_run_many',
    'analytics_check',
    'dashboards_list',
    'charts_list',
    'savings_overview',
    'savings_progress',
    'debts_list',
    'debt_schedule_preview',
    'settings_get',
    'prefs_get',
    'vault_status',
    'search',
    'query_parse',
    'app_version'
  ]) {
    expect(isReadCommand(read), read).toBe(true);
  }
  for (const write of [
    'tx_create',
    'tx_update',
    'tx_delete',
    'tx_restore',
    'tx_set_status',
    'plan_lock',
    'plan_unlock',
    'plan_wizard_apply',
    'plan_copy_from_previous',
    'plan_carry_forward',
    'categories_archive',
    'categories_create',
    'savings_fixed_set',
    'debt_payment_set_status',
    'incomes_create',
    'charts_update',
    'dashboards_create',
    'settings_set'
  ]) {
    expect(isReadCommand(write), write).toBe(false);
  }
});

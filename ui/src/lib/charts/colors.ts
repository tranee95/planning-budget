/** Цвета графиков: токены из `ChartData` → значения темы. Canvas не понимает `var()`. */

/** Читает значение CSS-переменной текущей темы, например `--ink`. */
export type CssReader = (name: string) => string;

export type CategoryColors = ReadonlyMap<number, string>;

const TOKEN_VARS: Record<string, string> = {
  'status.paid': '--paid-bar',
  'status.debt': '--debt-bar',
  'status.unplanned': '--unpl-bar',
  'status.planned': '--plan-bar',
  'kind.mandatory': '--kind-mandatory',
  'kind.wants': '--kind-wants',
  'kind.savings': '--kind-savings',
  'kind.loans': '--kind-loans',
  'metric.income': '--muted',
  'metric.expenses': '--ink',
  'metric.savings': '--accent',
  'metric.savings_cum': '--accent',
  'metric.savings_rate': '--accent',
  'metric.free': '--paid-bar',
  'metric.free_cum': '--paid-bar',
  'scenario.a': '--kind-mandatory',
  'scenario.b': '--kind-savings',
  'scenario.c': '--kind-wants',
  'bonds.balance': '--accent',
  'bonds.deposited': '--muted',
  other: '--plan-bar'
};

/** Серия прошлого периода: тот же цвет, что у основной (отличается штрихом и прозрачностью). */
export const PREV_SUFFIX = '.prev';

export function isPrevious(token: string): boolean {
  return token.endsWith(PREV_SUFFIX);
}

/** Прогноз: тот же цвет и штрих, но без прозрачности. */
export const FORECAST_SUFFIX = '.forecast';

export function isForecast(token: string): boolean {
  return token.endsWith(FORECAST_SUFFIX);
}

function withoutSuffix(token: string): string {
  if (isPrevious(token)) return token.slice(0, -PREV_SUFFIX.length);
  if (isForecast(token)) return token.slice(0, -FORECAST_SUFFIX.length);
  return token;
}

/**
 * Токен → цвет. `category:<id>` берёт цвет категории (он хранится в данных), неизвестное —
 * акцентный цвет.
 */
export function resolveColor(token: string, css: CssReader, categories: CategoryColors): string {
  const base = withoutSuffix(token);
  if (base.startsWith('category:')) {
    const color = categories.get(Number(base.slice('category:'.length)));
    return color ?? css('--plan-bar');
  }
  return css(TOKEN_VARS[base] ?? '--accent');
}

export function cssReader(root: Element = document.documentElement): CssReader {
  const style = getComputedStyle(root);
  return (name) => style.getPropertyValue(name).trim();
}

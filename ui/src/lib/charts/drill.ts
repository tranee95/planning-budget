import type { ChartDataDto, ChartSpecDto } from '$lib/api/bindings';

/** Клик по столбцу, сектору или точке → строка запроса для «Расходов». */

const KIND_WORDS: Record<string, string> = {
  mandatory: 'обязательные',
  wants: 'желания',
  savings: 'сбережения',
  loans: 'займы'
};
const STATUS_WORDS: Record<string, string> = {
  paid: 'оплачено',
  debt: 'долг',
  unplanned: 'внеплан',
  planned: 'план'
};

const quote = (name: string): string => `"${name.replaceAll('"', '')}"`;

/** Условие запроса для токена подписи или серии; `null` — токен ничего не сужает. */
function condition(token: string, label: string): string | null {
  if (token.startsWith('month:')) return `мес:${token.slice('month:'.length)}`;
  if (token.startsWith('kind.')) {
    const word = KIND_WORDS[token.slice('kind.'.length)];
    return word === undefined ? null : `тип:${word}`;
  }
  if (token.startsWith('status.')) {
    const word = STATUS_WORDS[token.slice('status.'.length)];
    return word === undefined ? null : `статус:${word}`;
  }
  if (token.startsWith('category:')) return `кат:${quote(label)}`;
  return label.startsWith('#') && label.length > 1 ? label : null;
}

const pad = (n: number): string => String(n).padStart(2, '0');

/** Период графика, если он не месяц оси: пресеты привязаны к текущей дате и обрываются на ней. */
function periodCondition(spec: ChartSpecDto, now: Date): string | null {
  const period = spec.period;
  if (period.preset === undefined) return `период:${period.from}..${period.to}`;
  const last = `${String(now.getFullYear())}-${pad(now.getMonth() + 1)}`;
  switch (period.preset) {
    case 'ytd':
      return `период:${String(now.getFullYear())}-01..${last}`;
    case 'current_month':
      return `мес:${last}`;
    case 'last12': {
      const start = new Date(now.getFullYear(), now.getMonth() - 11, 1);
      return `период:${String(start.getFullYear())}-${pad(start.getMonth() + 1)}..${last}`;
    }
    case 'all':
      return null;
  }
}

/** Показатели «Расходы» и «Сбережения» считают только часть типов: список тоже должен. */
function metricKinds(spec: ChartSpecDto): string | null {
  if (spec.metric === 'expenses') return 'тип:обязательные,желания,займы';
  if (spec.metric === 'savings') return 'тип:сбережения';
  return null;
}

/**
 * Запрос, показывающий записи за выбранный элемент графика: фильтр графика, период (если ось не
 * месяцы) и условия по подписи и серии. `null` — у этого графика нет перехода к записям.
 */
export function drillQuery(
  spec: ChartSpecDto,
  data: ChartDataDto,
  categoryIndex: number,
  seriesIndex: number,
  now: Date = new Date()
): string | null {
  const series = data.series[seriesIndex];
  // Доходы живут в другой таблице: переход к тратам по ним был бы обманом
  if (series?.color === 'metric.income' || (spec.metric === 'income' && spec.seriesBy === null)) {
    return null;
  }
  const parts: string[] = [];
  if (spec.filter.trim() !== '') parts.push(spec.filter.trim());

  const label = data.categories[categoryIndex] ?? '';
  const axis = condition(data.categoryTokens[categoryIndex] ?? '', label);
  if (axis !== null) parts.push(axis);
  if (axis?.startsWith('мес:') !== true) {
    const period = periodCondition(spec, now);
    if (period !== null) parts.push(period);
  }

  if (series !== undefined && spec.seriesBy !== null && spec.seriesBy !== 'metric') {
    const extra = condition(series.color, series.name);
    if (extra !== null) parts.push(extra);
  }
  if (!parts.some((p) => p.startsWith('тип:'))) {
    const kinds = metricKinds(spec);
    if (kinds !== null) parts.push(kinds);
  }
  return parts.join(' ');
}

import { events, type ChangeScope, type DataChanged } from '$lib/api/bindings';

/** Окно коалесценции: события одной правки (несколько команд подряд, импорт) приходят пачкой. */
export const COALESCE_MS = 25;

/** Изменение, возможно склеенное из нескольких событий; `receivedAt` — время последнего из них. */
export interface DataChange extends DataChanged {
  receivedAt: number;
}

type Handler = (change: DataChange) => void;

interface Subscriber {
  handler: Handler;
  scopes: readonly ChangeScope[] | undefined;
}

/** Склейка: разные области дают `all`, пустой список месяцев («все») поглощает остальные. */
export function mergeChanges(a: DataChange | undefined, b: DataChange): DataChange {
  if (a === undefined) return b;
  const months =
    a.months.length === 0 || b.months.length === 0 ? [] : [...new Set([...a.months, ...b.months])];
  return { scope: a.scope === b.scope ? a.scope : 'all', months, receivedAt: b.receivedAt };
}

const subscribers = new Set<Subscriber>();
let stopListening: Promise<() => void> | undefined;
let pending: DataChange | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;

function flush(): void {
  const change = pending;
  pending = undefined;
  timer = undefined;
  if (change === undefined) return;
  for (const { handler, scopes } of [...subscribers]) {
    if (scopes === undefined || change.scope === 'all' || scopes.includes(change.scope)) {
      handler(change);
    }
  }
}

function ensureListening(): void {
  stopListening ??= events.dataChanged.listen(({ payload }) => {
    pending = mergeChanges(pending, { ...payload, receivedAt: performance.now() });
    timer ??= setTimeout(flush, COALESCE_MS);
  });
}

function stopIfIdle(): void {
  if (subscribers.size > 0 || stopListening === undefined) return;
  const off = stopListening;
  stopListening = undefined;
  clearTimeout(timer);
  timer = undefined;
  pending = undefined;
  void off.then((stop) => {
    stop();
  });
}

/**
 * Подписка на `data-changed`. Одна общая подписка на всё приложение:
 * события в пределах `COALESCE_MS` склеиваются, обработчик вызывается один раз.
 * Без `scopes` обработчик получает все изменения; с `scopes` — только перечисленные области
 * и `all`. Возвращает синхронный cleanup.
 */
export function onDataChanged(handler: Handler, scopes?: readonly ChangeScope[]): () => void {
  const sub: Subscriber = { handler, scopes };
  subscribers.add(sub);
  ensureListening();
  return () => {
    subscribers.delete(sub);
    stopIfIdle();
  };
}

/**
 * Метка последней загрузки экрана. Если загрузка началась после изменения (явный `load` после
 * команды), данные уже свежие и перечитывать их по событию не нужно.
 */
export class LoadStamp {
  #startedAt = -Infinity;

  mark(): void {
    this.#startedAt = performance.now();
  }

  isFresh(change: DataChange): boolean {
    return this.#startedAt >= change.receivedAt;
  }
}

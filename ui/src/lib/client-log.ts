import type { ClientErrorDto } from '$lib/api/bindings';
import { ApiError } from '$lib/api/call';
import { clientLogApi } from '$lib/api/data';

/**
 * Ошибки интерфейса в журнал приложения. Страница «500 Internal Error» и
 * исключения вне обработчиков мелькают на секунду, поэтому текст сбоя остаётся только в журнале.
 *
 * В журнал не попадают данные бюджета: текст ошибки уходит только у встроенных типов (их
 * сообщения описывают код, а не данные) и у `ApiError` (там только код ошибки); у остальных
 * текст пустой. Сжимает, обрезает и проверяет поля ещё раз Rust.
 */

type Kind = ClientErrorDto['kind'];
type Send = (entry: ClientErrorDto) => Promise<unknown>;

const BUILT_IN = new Set([
  'TypeError',
  'ReferenceError',
  'RangeError',
  'SyntaxError',
  'EvalError',
  'URIError'
]);

/** Сколько разных ошибок отправляется за жизнь страницы: цикл сбоев не должен засыпать журнал. */
const MAX_REPORTS = 30;

const seen = new Set<string>();

/** Безопасное описание ошибки: тип, текст (только у встроенных типов) и кадры стека. */
export function describeError(error: unknown): Pick<ClientErrorDto, 'name' | 'message' | 'stack'> {
  if (error instanceof ApiError) {
    return { name: 'ApiError', message: error.error.code, stack: '' };
  }
  if (error instanceof Error) {
    const message = BUILT_IN.has(error.name) ? error.message : '';
    return { name: error.name, message, stack: error.stack ?? '' };
  }
  return { name: typeof error, message: '', stack: '' };
}

/** Путь без параметров запроса и якоря: `/expenses?q=лента` → `/expenses`. */
export function routeOf(href: string): string {
  const path = href.replace(/^[a-z][a-z0-9+.-]*:\/\/[^/]*/i, '');
  return path.split(/[?#]/)[0] || '/';
}

export function reportClientError(
  kind: Kind,
  error: unknown,
  route: string,
  send: Send = clientLogApi.error
): void {
  const entry: ClientErrorDto = { kind, ...describeError(error), route: routeOf(route) };
  const key = `${entry.kind}|${entry.name}|${entry.message}|${entry.route}`;
  if (seen.has(key) || seen.size >= MAX_REPORTS) return;
  seen.add(key);
  // Журнал вторичен: сбой самой отправки (например, нет моста IPC) ничего не ломает.
  void send(entry).catch(() => undefined);
}

/** Подписка на необработанные исключения и отказы промисов окна. Возвращает отписку. */
export function installClientErrorLog(send?: Send): () => void {
  const onError = (event: ErrorEvent): void => {
    reportClientError('uncaught', event.error ?? new Error(event.message), location.href, send);
  };
  const onRejection = (event: PromiseRejectionEvent): void => {
    reportClientError('rejection', event.reason, location.href, send);
  };
  window.addEventListener('error', onError);
  window.addEventListener('unhandledrejection', onRejection);
  return () => {
    window.removeEventListener('error', onError);
    window.removeEventListener('unhandledrejection', onRejection);
  };
}

/** Для тестов: забыть уже отправленные ошибки. */
export function resetClientLog(): void {
  seen.clear();
}

import type { AppError } from './bindings';

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError };

/** Ошибка команды: бэкенд отдаёт только код и ключи, текст подбирает `lib/i18n`. */
export class ApiError extends Error {
  constructor(readonly error: AppError) {
    super(error.code);
    this.name = 'ApiError';
  }
}

let lockedHandler: (() => void) | undefined;

/** Сессия регистрирует сюда реакцию на `Locked` из любой команды. */
export function onBackendLocked(handler: () => void): void {
  lockedHandler = handler;
}

/** Единая обёртка над `commands.*`: успех → данные, ошибка → `ApiError`. */
export async function call<T>(result: Promise<Result<T>>): Promise<T> {
  const r = await result;
  if (r.status === 'ok') return r.data;
  if (r.error.code === 'Locked') lockedHandler?.();
  throw new ApiError(r.error);
}

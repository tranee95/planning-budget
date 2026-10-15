import type { HandleClientError } from '@sveltejs/kit';
import { reportClientError } from '$lib/client-log';

/**
 * Любая ошибка при открытии маршрута (не загрузился модуль экрана, исключение в `load` или при
 * отрисовке) ведёт на страницу «500 Internal Error»; причина уходит в журнал приложения.
 */
export const handleError: HandleClientError = ({ error, event }) => {
  reportClientError('route', error, event.url.pathname);
};

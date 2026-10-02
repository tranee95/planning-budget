import { error } from '@sveltejs/kit';

export function load(): void {
  // Витрина компонентов нужна только при разработке: в сборке маршрут недоступен.
  if (!import.meta.env.DEV) error(404, 'Не найдено');
}

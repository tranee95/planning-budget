import type { Prefs, PrefsPatch } from '$lib/api/bindings';
import { events } from '$lib/api/bindings';
import { prefsApi } from '$lib/api/prefs';

/**
 * Переносит настройки на `<html>`: тему, масштаб и режим «меньше анимаций».
 * `systemDark` — тема ОС, которую сообщило окно: WebView2 не всегда передаёт её странице через
 * `prefers-color-scheme`, поэтому «системная» тема разрешается в явную светлую или тёмную.
 * Пока тема ОС неизвестна (`null`), остаётся только media-запрос из токенов.
 */
export function applyPrefs(
  prefs: Prefs,
  root: HTMLElement = document.documentElement,
  systemDark: boolean | null = null
): void {
  if (prefs.theme === 'system') {
    if (systemDark === null) delete root.dataset.theme;
    else root.dataset.theme = systemDark ? 'dark' : 'light';
  } else root.dataset.theme = prefs.theme;
  if (prefs.reducedMotion === 'on') root.dataset.motion = 'reduce';
  else delete root.dataset.motion;
  // CSS zoom не масштабирует единицы vh: высоту «на весь экран» делим на масштаб (--screen-h).
  const scale = prefs.uiScale === 100 ? '' : String(prefs.uiScale / 100);
  root.style.zoom = scale;
  if (scale) root.style.setProperty('--ui-zoom', scale);
  else root.style.removeProperty('--ui-zoom');
}

class PrefsStore {
  current = $state.raw<Prefs | null>(null);
  #systemDark: boolean | null = null;
  #subscribed = false;

  /** Читает `ui-prefs.json`: работает до разблокировки. Ошибка не мешает старту. */
  async load(): Promise<void> {
    // Подписка раньше первого чтения: смена темы ОС во время загрузки не теряется.
    this.#subscribe();
    try {
      this.current = await prefsApi.get();
      await this.#refreshSystemTheme();
      applyPrefs(this.current, document.documentElement, this.#systemDark);
    } catch {
      // оболочка остаётся со значениями по умолчанию
    }
  }

  async set(patch: PrefsPatch): Promise<void> {
    this.current = await prefsApi.set(patch);
    await this.#refreshSystemTheme();
    applyPrefs(this.current, document.documentElement, this.#systemDark);
  }

  /** Тема ОС изменилась, пока приложение открыто: «системная» следует ей. Один раз за жизнь страницы. */
  #subscribe(): void {
    if (this.#subscribed) return;
    this.#subscribed = true;
    void events.systemThemeChanged.listen((event) => {
      this.#systemDark = event.payload.dark;
      this.#apply();
    });
    // В браузере (dev:mock) событий от окна нет: следим за media-запросом и спрашиваем тему заново.
    if (typeof globalThis.matchMedia !== 'function') return;
    globalThis.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      void this.#refreshSystemTheme().then(() => {
        this.#apply();
      });
    });
  }

  #apply(): void {
    if (this.current) applyPrefs(this.current, document.documentElement, this.#systemDark);
  }

  async #refreshSystemTheme(): Promise<void> {
    try {
      this.#systemDark = await prefsApi.systemDark();
    } catch {
      this.#systemDark = null;
    }
  }
}

export const prefs = new PrefsStore();

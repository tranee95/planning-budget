//! Окно оболочки: фон до первой отрисовки и синхронизация WebView с размером окна.

use tauri::window::Color;
use tauri::{AppHandle, Manager, PhysicalPosition, Webview, WebviewWindow, Window, WindowEvent};

use tauri_specta::Event as _;

use crate::dto::Theme;
use crate::events::SystemThemeChanged;
use crate::prefs;

/// `--bg` светлой и тёмной тем.
const LIGHT_BG: Color = Color(0xF4, 0xF3, 0xEF, 0xFF);
const DARK_BG: Color = Color(0x0E, 0x0F, 0x11, 0xFF);

/// Цвет, который виден до первой отрисовки и в любой щели между окном и WebView.
/// `system` — тема ОС, а если её узнать нельзя, тёмная: вспышка тёмного не режет глаза.
#[must_use]
pub fn background_for(theme: &Theme, system: Option<tauri::Theme>) -> Color {
    match (theme, system) {
        (Theme::Light, _) | (Theme::System, Some(tauri::Theme::Light)) => LIGHT_BG,
        _ => DARK_BG,
    }
}

/// Тема окна: заголовок и цвет фона. `system` отдаёт заголовок системе, `light` и `dark` задают его явно,
/// иначе заголовок остаётся тёмным при светлой теме приложения (и наоборот).
pub fn apply_theme(app: &AppHandle, theme: &Theme) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let forced = match theme {
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
        Theme::System => None,
    };
    if let Err(err) = window.set_theme(forced) {
        tracing::warn!(%err, "window theme not applied");
    }
    let color = background_for(theme, window.theme().ok());
    if let Err(err) = window.set_background_color(Some(color)) {
        tracing::warn!(%err, "window background not applied");
    }
}

/// Тёмная ли тема у окна (в режиме «системная» — тема ОС). Если узнать нельзя — тёмная.
#[must_use]
pub fn is_dark(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.theme().ok())
        .is_none_or(|t| t == tauri::Theme::Dark)
}

/// Прямоугольник в физических пикселях.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Вписывает окно в рабочую область монитора: размер не больше области, положение внутри неё.
/// Нужна для сохранённого состояния, записанного на другом, большем мониторе.
#[must_use]
pub fn fit_into(window: Rect, area: Rect) -> Rect {
    let w = window.w.min(area.w);
    let h = window.h.min(area.h);
    let max_x = area.x.saturating_add_unsigned(area.w - w);
    let max_y = area.y.saturating_add_unsigned(area.h - h);
    Rect {
        x: window.x.clamp(area.x, max_x),
        y: window.y.clamp(area.y, max_y),
        w,
        h,
    }
}

/// Допуск для `needs_fit`, физические пиксели.
const FRAME_TOLERANCE: u32 = 16;

/// Окно нужно вписывать, только если оно заметно выходит за область. У окна Windows внешний
/// прямоугольник включает невидимые рамки (около 8 px с каждой стороны): окно вплотную к краю
/// экрана не должно сдвигаться при каждом запуске.
#[must_use]
pub fn needs_fit(window: Rect, area: Rect, tolerance: u32) -> bool {
    let t = i64::from(tolerance);
    let (x, y) = (i64::from(window.x), i64::from(window.y));
    let (ax, ay) = (i64::from(area.x), i64::from(area.y));
    x < ax - t
        || y < ay - t
        || x + i64::from(window.w) > ax + i64::from(area.w) + t
        || y + i64::from(window.h) > ay + i64::from(area.h) + t
}

fn fit_to_monitor(window: &WebviewWindow) {
    if window.is_maximized().unwrap_or(false) {
        return;
    }
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(pos), Ok(outer), Ok(inner)) = (
        monitor,
        window.outer_position(),
        window.outer_size(),
        window.inner_size(),
    ) else {
        return;
    };
    let area = monitor.work_area();
    let current = Rect {
        x: pos.x,
        y: pos.y,
        w: outer.width,
        h: outer.height,
    };
    let work = Rect {
        x: area.position.x,
        y: area.position.y,
        w: area.size.width,
        h: area.size.height,
    };
    if !needs_fit(current, work, FRAME_TOLERANCE) {
        return;
    }
    let fitted = fit_into(current, work);
    // set_size задаёт клиентскую область: вычитаем рамку и заголовок.
    let frame_w = outer.width.saturating_sub(inner.width);
    let frame_h = outer.height.saturating_sub(inner.height);
    let size = tauri::PhysicalSize::new(
        fitted.w.saturating_sub(frame_w),
        fitted.h.saturating_sub(frame_h),
    );
    if let Err(err) = window
        .set_size(size)
        .and_then(|()| window.set_position(tauri::PhysicalPosition::new(fitted.x, fitted.y)))
    {
        tracing::warn!(%err, "window not fitted to the monitor");
    }
}

/// Красит окно по сохранённой теме и вписывает в монитор. Вызывается из `setup`, до показа содержимого.
pub fn init(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let path = app.state::<crate::state::AppState>().paths().prefs();
    apply_theme(app, &prefs::load(&path).theme);
    fit_to_monitor(&window);
    sync_webview(&window);
}

/// WebView обязан занимать всю клиентскую область. После восстановления сохранённого размера
/// и при смене масштаба (перенос на монитор с другим DPI) WebView2 мог остаться прежнего
/// размера, а остальное окно заливалось белым: размер выставляем явно.
pub fn sync_webview(window: &WebviewWindow) {
    let webview: &Webview = window.as_ref();
    let Ok(size) = window.inner_size() else {
        return;
    };
    if size.width == 0 || size.height == 0 {
        return; // свёрнутое окно
    }
    if let Err(err) = webview
        .set_position(PhysicalPosition::new(0, 0))
        .and_then(|()| webview.set_size(size))
    {
        tracing::warn!(%err, "webview resize not applied");
    }
}

pub fn on_event(window: &Window, event: &WindowEvent) {
    // Событие темы может прийти до `setup`, когда `AppState` ещё не создан: тогда ничего не делаем.
    if let WindowEvent::ThemeChanged(theme) = event
        && let Some(state) = window.app_handle().try_state::<crate::state::AppState>()
    {
        let app = window.app_handle();
        if let Err(err) = (SystemThemeChanged {
            dark: *theme == tauri::Theme::Dark,
        })
        .emit(app)
        {
            tracing::warn!(%err, "SystemThemeChanged emit failed");
        }
        // В режиме «системная» фон окна следует теме ОС.
        let path = state.paths().prefs();
        apply_theme(app, &prefs::load(&path).theme);
    }
    // Свёрнутое окно блокирует сейф раньше, чем содержимое попадёт в миниатюру переключателя задач.
    // Отдельного события «свёрнуто» у Tauri нет: на Windows приходит `Resized`, на других системах
    // сворачивание может прийти потерей фокуса. Проверено на Windows.
    if matches!(event, WindowEvent::Resized(_) | WindowEvent::Focused(false))
        && window.is_minimized().unwrap_or(false)
        && let Some(state) = window.app_handle().try_state::<crate::state::AppState>()
        && state.lock_on_minimize()
    {
        crate::events::lock_session(window.app_handle(), crate::events::LockReason::Minimize);
    }
    if matches!(
        event,
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
    ) && let Some(webview_window) = window.app_handle().get_webview_window(window.label())
    {
        sync_webview(&webview_window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        w: 2560,
        h: 1400,
    };

    #[test]
    fn background_follows_saved_theme() {
        assert_eq!(
            background_for(&Theme::Light, Some(tauri::Theme::Dark)),
            LIGHT_BG
        );
        assert_eq!(
            background_for(&Theme::Dark, Some(tauri::Theme::Light)),
            DARK_BG
        );
    }

    #[test]
    fn system_theme_uses_the_os_theme_and_falls_back_to_dark() {
        assert_eq!(
            background_for(&Theme::System, Some(tauri::Theme::Light)),
            LIGHT_BG
        );
        assert_eq!(
            background_for(&Theme::System, Some(tauri::Theme::Dark)),
            DARK_BG
        );
        assert_eq!(background_for(&Theme::System, None), DARK_BG);
    }

    #[test]
    fn window_inside_the_area_is_left_alone() {
        let w = Rect {
            x: 100,
            y: 100,
            w: 1280,
            h: 800,
        };
        assert_eq!(fit_into(w, AREA), w);
    }

    #[test]
    fn window_bigger_than_the_area_is_shrunk_to_it() {
        let w = Rect {
            x: 0,
            y: 0,
            w: 6018,
            h: 3247,
        };
        assert_eq!(fit_into(w, AREA), AREA);
    }

    #[test]
    fn window_off_the_screen_is_moved_back() {
        let w = Rect {
            x: -9000,
            y: 120,
            w: 1280,
            h: 800,
        };
        assert_eq!(
            fit_into(w, AREA),
            Rect {
                x: 0,
                y: 120,
                w: 1280,
                h: 800
            }
        );
        let w = Rect {
            x: 2400,
            y: 1300,
            w: 1280,
            h: 800,
        };
        assert_eq!(
            fit_into(w, AREA),
            Rect {
                x: 1280,
                y: 600,
                w: 1280,
                h: 800
            }
        );
    }

    #[test]
    fn area_with_an_offset_is_respected() {
        let area = Rect {
            x: 100,
            y: 50,
            w: 1000,
            h: 700,
        };
        let w = Rect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        };
        assert_eq!(fit_into(w, area), area);
    }
}

#[cfg(test)]
mod fit_tolerance_tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        w: 2560,
        h: 1400,
    };

    #[test]
    fn invisible_window_frames_do_not_trigger_a_fit() {
        // Окно вплотную к краю: внешний прямоугольник выступает на 8 px рамки.
        let flush_left = Rect {
            x: -8,
            y: 0,
            w: 1296,
            h: 808,
        };
        assert!(!needs_fit(flush_left, AREA, FRAME_TOLERANCE));
        let flush_right = Rect {
            x: 2560 - 1296 + 8,
            y: 0,
            w: 1296,
            h: 808,
        };
        assert!(!needs_fit(flush_right, AREA, FRAME_TOLERANCE));
    }

    #[test]
    fn a_clearly_oversized_or_offscreen_window_needs_a_fit() {
        assert!(needs_fit(
            Rect {
                x: 0,
                y: 0,
                w: 6018,
                h: 3247
            },
            AREA,
            FRAME_TOLERANCE
        ));
        assert!(needs_fit(
            Rect {
                x: -9000,
                y: 120,
                w: 1280,
                h: 800
            },
            AREA,
            FRAME_TOLERANCE
        ));
        assert!(needs_fit(
            Rect {
                x: 2400,
                y: 1300,
                w: 1280,
                h: 800
            },
            AREA,
            FRAME_TOLERANCE
        ));
    }
}

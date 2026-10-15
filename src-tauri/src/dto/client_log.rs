//! Ошибки интерфейса, которые фронтенд передаёт в журнал приложения.

use serde::Deserialize;
use specta::Type;

/// Откуда пришла ошибка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ClientErrorKind {
    /// Не открылся маршрут: SvelteKit показал страницу «500 Internal Error».
    Route,
    /// Необработанное исключение (`window.onerror`).
    Uncaught,
    /// Необработанный отказ промиса (`unhandledrejection`).
    Rejection,
}

/// Техническое описание ошибки без данных бюджета: фронтенд отправляет сообщение только у
/// встроенных типов ошибок, а Rust ещё раз чистит и обрезает все поля (`services::client_log`).
#[derive(Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClientErrorDto {
    pub kind: ClientErrorKind,
    /// Имя типа ошибки: `TypeError`, `Error` и т. п.
    pub name: String,
    /// Текст ошибки; пустой, если тип пользовательский и текст мог содержать данные.
    pub message: String,
    /// Кадры стека (`at …`), не больше восьми.
    pub stack: String,
    /// Путь маршрута без параметров запроса: `/analytics`.
    pub route: String,
}

// Текст пришёл с фронтенда и ещё не очищен: в `{:?}` он не попадает.
impl std::fmt::Debug for ClientErrorDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientErrorDto")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

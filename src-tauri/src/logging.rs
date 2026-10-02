//! Логи в файл с суточной ротацией, хранятся 7 дней.
//! В логи попадают только имена команд, id, коды ошибок и тайминги, но не суммы и названия.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{InitError, Rotation};

/// Включает запись логов в `dir`. Guard надо держать до выхода из приложения.
///
/// # Errors
/// Если каталог логов недоступен для записи.
pub fn init(dir: &Path) -> Result<WorkerGuard, InitError> {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("app")
        .filename_suffix("log")
        .max_log_files(7)
        .build(dir)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    // try_init: повторная инициализация (например, в тестах) не должна падать
    let _ = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_max_level(level)
        .try_init();
    Ok(guard)
}

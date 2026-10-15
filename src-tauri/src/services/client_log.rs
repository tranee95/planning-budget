//! Запись ошибок интерфейса в журнал приложения.
//!
//! Фронтенд присылает техническое описание сбоя (тип ошибки, текст встроенных ошибок, кадры
//! стека, путь маршрута). Rust не доверяет ему: имена и путь оставляют только безопасные
//! символы, текст сжимается в одну строку и обрезается, записей за запуск не больше
//! `MAX_PER_RUN`, чтобы цикл ошибок не раздул журнал.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::dto::{ClientErrorDto, ClientErrorKind};

/// Сколько ошибок интерфейса попадает в журнал за один запуск.
const MAX_PER_RUN: u32 = 100;
const MAX_NAME: usize = 40;
const MAX_ROUTE: usize = 80;
const MAX_MESSAGE: usize = 240;
const MAX_FRAMES: usize = 8;
const MAX_FRAME: usize = 160;

static LOGGED: AtomicU32 = AtomicU32::new(0);

/// Поля записи после очистки.
#[derive(Debug, PartialEq, Eq)]
pub struct Clean {
    pub kind: &'static str,
    pub name: String,
    pub route: String,
    pub message: String,
    pub stack: String,
}

fn kind_name(kind: ClientErrorKind) -> &'static str {
    match kind {
        ClientErrorKind::Route => "route",
        ClientErrorKind::Uncaught => "uncaught",
        ClientErrorKind::Rejection => "rejection",
    }
}

/// Одна строка без управляющих символов (в журнале не должно быть разрыва записи).
fn one_line(text: &str, max: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    flat.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

fn keep_only(text: &str, max: usize, allowed: impl Fn(char) -> bool) -> String {
    text.chars().filter(|c| allowed(*c)).take(max).collect()
}

/// Чистит запись; сами значения формирует фронтенд, но они проходят проверку здесь.
#[must_use]
pub fn clean(entry: &ClientErrorDto) -> Clean {
    let path = entry.route.split(['?', '#']).next().unwrap_or_default();
    let route = keep_only(path, MAX_ROUTE, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.')
    });
    let stack = entry
        .stack
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("at ") || line.contains('@'))
        .take(MAX_FRAMES)
        .map(|line| one_line(line, MAX_FRAME))
        .collect::<Vec<_>>()
        .join(" | ");
    Clean {
        kind: kind_name(entry.kind),
        name: keep_only(&entry.name, MAX_NAME, |c| {
            c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '.')
        }),
        route: if route.is_empty() {
            "?".to_owned()
        } else {
            route
        },
        message: one_line(&entry.message, MAX_MESSAGE),
        stack,
    }
}

fn record_with(counter: &AtomicU32, entry: &ClientErrorDto) -> bool {
    let n = counter.fetch_add(1, Ordering::Relaxed);
    if n >= MAX_PER_RUN {
        return false;
    }
    let c = clean(entry);
    tracing::warn!(
        kind = c.kind,
        name = %c.name,
        route = %c.route,
        message = %c.message,
        stack = %c.stack,
        "client error"
    );
    if n + 1 == MAX_PER_RUN {
        tracing::warn!("client error log limit reached: further errors are not recorded");
    }
    true
}

/// Пишет ошибку интерфейса в журнал. `false` — лимит запуска исчерпан.
pub fn record(entry: &ClientErrorDto) -> bool {
    record_with(&LOGGED, entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> ClientErrorDto {
        ClientErrorDto {
            kind: ClientErrorKind::Route,
            name: "TypeError".to_owned(),
            message: "Failed to fetch dynamically imported module".to_owned(),
            stack: "TypeError: x\n    at load (tauri://localhost/_app/chunks/a.js:1:2)\nnoise"
                .to_owned(),
            route: "/analytics".to_owned(),
        }
    }

    #[test]
    fn clean_keeps_safe_fields_and_only_stack_frames() {
        let c = clean(&entry());
        assert_eq!(c.kind, "route");
        assert_eq!(c.name, "TypeError");
        assert_eq!(c.route, "/analytics");
        assert_eq!(c.message, "Failed to fetch dynamically imported module");
        assert_eq!(c.stack, "at load (tauri://localhost/_app/chunks/a.js:1:2)");
    }

    #[test]
    fn route_drops_query_and_unsafe_characters() {
        let mut e = entry();
        e.route = "/expenses?q=лента сумма>5000#x".to_owned();
        assert_eq!(clean(&e).route, "/expenses");
        e.route = "?q=секрет".to_owned();
        assert_eq!(clean(&e).route, "?");
    }

    #[test]
    fn message_is_one_truncated_line_without_control_characters() {
        let mut e = entry();
        e.message = format!("первая\nвторая\r\n\tтретья {}", "x".repeat(400));
        let c = clean(&e);
        assert!(!c.message.contains(['\n', '\r', '\t']));
        assert!(c.message.starts_with("первая вторая третья"));
        assert_eq!(c.message.chars().count(), MAX_MESSAGE);
    }

    #[test]
    fn name_and_stack_are_bounded() {
        let mut e = entry();
        e.name = format!("Evil Name\n{}", "a".repeat(100));
        e.stack = (0..20)
            .map(|i| format!("at f{i} (a.js:1:1)"))
            .collect::<Vec<_>>()
            .join("\n");
        let c = clean(&e);
        assert!(c.name.len() <= MAX_NAME && !c.name.contains(' '));
        assert_eq!(c.stack.split(" | ").count(), MAX_FRAMES);
    }

    #[test]
    fn run_limit_stops_recording() {
        let counter = AtomicU32::new(0);
        let written = (0..MAX_PER_RUN + 5)
            .filter(|_| record_with(&counter, &entry()))
            .count();
        assert_eq!(written, MAX_PER_RUN as usize);
    }
}

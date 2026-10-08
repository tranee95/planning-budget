//! Недавние запросы на создание: повтор с тем же `request_id` возвращает прежний результат
//!. Живёт в сессии и пропадает при блокировке.

use std::collections::VecDeque;

use crate::AppError;

/// Сколько последних запросов помнит сессия.
const CAPACITY: usize = 64;
/// UUID занимает 36 символов; запас на другие форматы идентификатора.
const MAX_ID_LEN: usize = 64;

/// Ограниченный список `(request_id, результат)`; при переполнении вытесняется самый старый.
#[derive(Debug)]
pub struct RecentRequests<T> {
    items: VecDeque<(String, T)>,
}

impl<T: Clone> RecentRequests<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: VecDeque::with_capacity(CAPACITY),
        }
    }

    /// Результат ранее выполненного запроса с таким id.
    #[must_use]
    pub fn get(&self, request_id: &str) -> Option<T> {
        self.items
            .iter()
            .find(|(id, _)| id == request_id)
            .map(|(_, v)| v.clone())
    }

    pub fn remember(&mut self, request_id: String, value: T) {
        if self.items.len() == CAPACITY {
            self.items.pop_front();
        }
        self.items.push_back((request_id, value));
    }
}

impl<T: Clone> Default for RecentRequests<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Проверяет идентификатор запроса: непустой, ограниченной длины, без управляющих символов.
///
/// # Errors
/// `Validation` с полем `requestId`.
pub fn check_request_id(request_id: &str) -> Result<(), AppError> {
    if request_id.is_empty()
        || request_id.len() > MAX_ID_LEN
        || request_id.chars().any(char::is_control)
    {
        return Err(AppError::invalid_field("request_id_invalid", "requestId"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_what_was_remembered() {
        let mut recent = RecentRequests::new();
        assert_eq!(recent.get("a"), None::<i32>);
        recent.remember("a".into(), 1);
        assert_eq!(recent.get("a"), Some(1));
        assert_eq!(recent.get("b"), None);
    }

    #[test]
    fn forgets_the_oldest_when_full() {
        let mut recent = RecentRequests::new();
        for i in 0..=CAPACITY {
            recent.remember(format!("id-{i}"), i);
        }
        assert_eq!(recent.get("id-0"), None);
        assert_eq!(recent.get("id-1"), Some(1));
        assert_eq!(recent.get(&format!("id-{CAPACITY}")), Some(CAPACITY));
    }

    #[test]
    fn rejects_empty_long_and_control_ids() {
        assert!(check_request_id("0b8f6a3e-1c1d-4a52-9d57-2f6f6f5f3c11").is_ok());
        assert!(check_request_id("").is_err());
        assert!(check_request_id(&"x".repeat(MAX_ID_LEN + 1)).is_err());
        assert!(check_request_id("a\nb").is_err());
    }
}

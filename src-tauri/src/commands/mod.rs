//! Tauri-команды. Тонкие: сессия → сервис → DTO.
//! Лежат в модулях, а не в корне `lib.rs`: pub-команда в корне дублирует макросы `__cmd__*`.
//! Каждая команда, работающая с данными, ходит в БД только через `AppState::with_session`;
//! `tests/locked.rs` проверяет это по исходникам.

pub mod analytics;
pub mod app;
pub mod bonds;
pub mod categories;
pub mod dev;
pub mod incomes;
pub mod legacy;
pub mod prefs;
pub mod search;
pub mod settings;
pub mod summary;
pub mod transactions;
pub mod vault;

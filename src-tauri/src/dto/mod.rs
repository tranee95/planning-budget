//! DTO на границе IPC. Типы для фронтенда генерирует tauri-specta.
//! Модуль на предметную область; снаружи всё доступно как `crate::dto::<Тип>`.

mod analytics;
mod categories;
mod common;
mod debts;
mod legacy;
mod plan;
mod prefs;
mod savings;
mod search;
mod settings;
mod summary;
mod transactions;
mod vault;

pub use analytics::*;
pub use categories::*;
pub use common::*;
pub use debts::*;
pub use legacy::*;
pub use plan::*;
pub use prefs::*;
pub use savings::*;
pub use search::*;
pub use settings::*;
pub use summary::*;
pub use transactions::*;
pub use vault::*;

//! Dev-сид: заливка `testdata/seed-2026.json` в пустую базу. Данные фикстуры есть только
//! в debug-сборке: в релизе команда отвечает ошибкой, а файл в бинарник не попадает.

use budget_storage::Db;
use chrono::{DateTime, Utc};

use crate::AppError;

/// Заменяет категории и настройки расчётов данными сида и добавляет его траты и доходы.
/// Если в базе уже есть траты или доходы, возвращает `Conflict`: ничего не затирается.
#[cfg(debug_assertions)]
pub fn load(db: &mut Db, now: DateTime<Utc>) -> Result<(), AppError> {
    const SEED: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../testdata/seed-2026.json"
    ));
    let data = budget_core::load_seed(SEED)?;
    db.load_dataset(&data, now)?;
    Ok(())
}

#[cfg(not(debug_assertions))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "та же сигнатура, что у debug-варианта"
)]
pub fn load(_db: &mut Db, _now: DateTime<Utc>) -> Result<(), AppError> {
    Err(AppError::Validation {
        message_key: "errors.dev_only".into(),
        field: None,
    })
}

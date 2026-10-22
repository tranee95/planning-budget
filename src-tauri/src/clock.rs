//! Время запроса: одна точка, из которой команды берут «сейчас» (аудит 2026-10-15, A1).
//!
//! Метки изменений пишутся по UTC, а календарные границы («текущий месяц», «сегодня») — по
//! местной дате пользователя. Выводить одно из другого нельзя: в UTC+N первые N часов месяца по
//! UTC ещё прошлый.

use chrono::{DateTime, Datelike as _, Local, NaiveDate, Utc};
use planning_budget_core::YearMonth;

use crate::AppError;

/// Момент запроса: метка UTC и местная календарная дата одного и того же мгновения.
#[derive(Clone, Copy, Debug)]
pub struct Now {
    pub utc: DateTime<Utc>,
    pub today: NaiveDate,
}

impl Now {
    /// Берёт текущее время один раз; метка и дата не расходятся даже на границе суток.
    #[must_use]
    pub fn capture() -> Self {
        let local = Local::now();
        Self {
            utc: local.with_timezone(&Utc),
            today: local.date_naive(),
        }
    }

    /// Текущий месяц по местной дате.
    ///
    /// # Errors
    /// Год вне диапазона `YearMonth`.
    pub fn month(&self) -> Result<YearMonth, AppError> {
        YearMonth::new(self.year(), u8::try_from(self.today.month()).unwrap_or(1))
            .map_err(|_| AppError::invalid_field("month_invalid", "month"))
    }

    /// Местный календарный год; для года вне `u16` (дата далеко за пределами домена) — границы типа.
    #[must_use]
    pub fn year(&self) -> u16 {
        u16::try_from(self.today.year().max(0)).unwrap_or(u16::MAX)
    }
}

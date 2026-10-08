//! Задержка после неудачных попыток входа по паролю.

use chrono::{DateTime, Duration, Utc};

use crate::file::VaultFile;

const FREE_ATTEMPTS: u32 = 3;
const MAX_BACKOFF_SECS: u64 = 60;

/// Задержка после `failed_attempts` неудач подряд: `2^(n−3)` с после третьей, максимум 60 с.
#[must_use]
pub fn backoff_secs(failed_attempts: u32) -> u64 {
    if failed_attempts < FREE_ATTEMPTS {
        return 0;
    }
    let exp = failed_attempts - FREE_ATTEMPTS;
    if exp >= 6 {
        MAX_BACKOFF_SECS
    } else {
        (1u64 << exp).min(MAX_BACKOFF_SECS)
    }
}

/// Сколько секунд ещё ждать до следующей попытки (округление вверх).
pub(super) fn retry_after(file: &VaultFile, now: DateTime<Utc>) -> u64 {
    let delay = backoff_secs(file.failed_attempts);
    let (true, Some(last)) = (delay > 0, file.last_failed_at) else {
        return 0;
    };
    let Ok(delay_secs) = i64::try_from(delay) else {
        return MAX_BACKOFF_SECS;
    };
    let remaining_ms = (last + Duration::seconds(delay_secs) - now).num_milliseconds();
    if remaining_ms <= 0 {
        return 0;
    }
    // Часы ушли назад: ждём не дольше максимальной задержки.
    u64::try_from((remaining_ms + 999) / 1000)
        .unwrap_or(MAX_BACKOFF_SECS)
        .min(MAX_BACKOFF_SECS)
}

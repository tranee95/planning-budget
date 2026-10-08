//! Сейф: статус и recovery-код.

use serde::Serialize;
use specta::Type;
use std::fmt;

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatusDto {
    pub exists: bool,
    pub locked: bool,
    /// Сколько мс ещё ждать до следующей попытки входа; `0` — можно вводить.
    pub retry_after_ms: u32,
    /// Перенос данных из папки прежнего идентификатора не удался: его можно повторить.
    pub migration_failed: bool,
}

/// Recovery-код показывается один раз сразу после создания.
#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCodeDto {
    pub recovery_code: String,
}

impl fmt::Debug for RecoveryCodeDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryCodeDto(<redacted>)")
    }
}

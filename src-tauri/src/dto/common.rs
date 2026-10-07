//! Общие типы границы IPC: секреты, целые из JS `number`, различение «не передано» и `null`.

use secrecy::SecretString;
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use std::fmt;

/// Пароль или recovery-код с фронтенда. В `Debug` и логи не попадает.
#[derive(Type)]
#[specta(transparent)]
pub struct Secret(#[specta(type = String)] SecretString);

impl Secret {
    #[must_use]
    pub fn secret(&self) -> &SecretString {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(|s| Self(SecretString::from(s)))
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedDto {
    pub saved: bool,
}

/// Целое из JS `number` (id, копейки < 2^53) в аргументах команд: tauri-specta не принимает
/// `i64` как есть, а в TS нужен `number`, а не `bigint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(transparent)]
#[specta(transparent)]
pub struct Int53(#[specta(type = specta_typescript::Number)] pub i64);

/// Различает «поле не передано» и «передан null».
pub(super) fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

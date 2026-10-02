//! Recovery-код: 128 бит, base32 без паддинга, группы по 4 символа.

use std::fmt;

use data_encoding::BASE32_NOPAD;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use crate::crypto::random_array;
use crate::error::VaultError;

const CODE_BYTES: usize = 16;
const GROUP: usize = 4;

/// Код восстановления для показа пользователю. Показывается один раз.
pub struct RecoveryCode {
    raw: Zeroizing<[u8; CODE_BYTES]>,
}

impl RecoveryCode {
    pub(crate) fn generate() -> Result<Self, VaultError> {
        Ok(Self {
            raw: Zeroizing::new(random_array::<CODE_BYTES>()?),
        })
    }

    /// Байты, из которых выводится KEK.
    pub(crate) fn secret(&self) -> &[u8] {
        self.raw.as_slice()
    }

    /// Текст вида `K7QM-2XPA-…` для экрана и файла.
    #[must_use]
    pub fn display(&self) -> Zeroizing<String> {
        let encoded = Zeroizing::new(BASE32_NOPAD.encode(self.raw.as_slice()));
        let mut grouped =
            Zeroizing::new(String::with_capacity(encoded.len() + encoded.len() / GROUP));
        for (i, ch) in encoded.chars().enumerate() {
            if i > 0 && i % GROUP == 0 {
                grouped.push('-');
            }
            grouped.push(ch);
        }
        grouped
    }
}

impl fmt::Debug for RecoveryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryCode(<redacted>)")
    }
}

/// Разбирает введённый пользователем код: регистр, дефисы и пробелы не важны.
pub(crate) fn parse(input: &SecretString) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let cleaned: Zeroizing<String> = Zeroizing::new(
        input
            .expose_secret()
            .chars()
            .filter(|c| *c != '-' && !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect(),
    );
    let bytes = BASE32_NOPAD
        .decode(cleaned.as_bytes())
        .map(Zeroizing::new)
        .map_err(|_| VaultError::RecoveryCodeFormat)?;
    if bytes.len() == CODE_BYTES {
        Ok(bytes)
    } else {
        Err(VaultError::RecoveryCodeFormat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_has_seven_groups_of_four_and_a_tail() {
        let code = RecoveryCode::generate().unwrap();
        let text = code.display();
        let groups: Vec<&str> = text.split('-').collect();
        assert_eq!(groups.len(), 7);
        assert!(groups[..6].iter().all(|g| g.len() == 4));
        assert_eq!(groups[6].len(), 2);
    }

    #[test]
    fn parse_roundtrips_displayed_code() {
        let code = RecoveryCode::generate().unwrap();
        let typed = SecretString::from(code.display().to_lowercase().replace('-', " "));
        assert_eq!(parse(&typed).unwrap().as_slice(), code.secret());
    }

    #[test]
    fn parse_rejects_garbage_and_wrong_length() {
        for bad in ["", "AAAA-BBBB", "11111111111111111111111111", "!!!!"] {
            let err = parse(&SecretString::from(bad)).unwrap_err();
            assert!(matches!(err, VaultError::RecoveryCodeFormat), "{bad}");
        }
    }
}

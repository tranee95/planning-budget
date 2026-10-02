use std::io;

/// Ошибки сейфа. Тексты без PII: пароли, ключи и пути сюда не попадают.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("wrong password")]
    WrongPassword,
    #[error("wrong recovery code")]
    WrongRecoveryCode,
    #[error("too many attempts, retry in {retry_after_secs} s")]
    Backoff { retry_after_secs: u64 },
    #[error("password is shorter than {min} characters")]
    PasswordTooShort { min: usize },
    #[error("recovery code has invalid format")]
    RecoveryCodeFormat,
    #[error("vault already exists")]
    AlreadyExists,
    #[error("key reissue is not finished")]
    RekeyPending,
    #[error("vault not found")]
    NotFound,
    #[error("vault file is corrupted")]
    Corrupt,
    #[error("unsupported vault version {0}")]
    UnsupportedVersion(u32),
    #[error("key derivation failed")]
    Kdf,
    #[error("encryption failed")]
    Crypto,
    #[error("random generator unavailable")]
    Random,
    #[error("io error: {0:?}")]
    Io(io::ErrorKind),
}

impl From<io::Error> for VaultError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.kind())
    }
}

//! Сейф: KDF, обёртка ключей, recovery-код.

mod crypto;
mod error;
mod file;
mod recovery;
mod store;

pub use crypto::{Dek, KdfParams, calibrate};
pub use error::VaultError;
pub use recovery::RecoveryCode;
pub use store::{Created, MIN_PASSWORD_CHARS, RekeyPlan, VaultStatus, VaultStore, backoff_secs};

//! Argon2id → KEK, XChaCha20-Poly1305 wrap/unwrap DEK.

use std::fmt;
use std::time::{Duration, Instant};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use data_encoding::BASE64;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

use crate::error::VaultError;

pub(crate) const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;
const AAD_PREFIX: &str = "budget-vault-v1|";

const CALIBRATION_FLOOR: Duration = Duration::from_millis(250);
const CALIBRATION_TARGET_MICROS: u128 = 500_000;
const MAX_T: u32 = 8;
/// Верхние границы при чтении `vault.json`: подделанный файл не должен
/// заставлять Argon2 съесть всю память или время.
const MAX_M_KIB: u32 = 1 << 20;
const MAX_P: u32 = 16;

/// Ключ шифрования данных (DEK). Живёт только в `Zeroizing`, не клонируется.
pub struct Dek(Zeroizing<[u8; KEY_LEN]>);

impl Dek {
    pub(crate) fn random() -> Result<Self, VaultError> {
        let mut bytes = Zeroizing::new([0u8; KEY_LEN]);
        fill_random(bytes.as_mut_slice())?;
        Ok(Self(bytes))
    }

    /// Сырые байты ключа — для `PRAGMA key` в `storage`.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl fmt::Debug for Dek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Dek(<redacted>)")
    }
}

/// Параметры Argon2id (хранятся в `vault.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl KdfParams {
    /// 64 MiB, 3 прохода, 1 поток.
    pub const DEFAULT: Self = Self {
        m_kib: 64 * 1024,
        t: 3,
        p: 1,
    };

    pub(crate) fn validate(self) -> Result<(), VaultError> {
        let ok = self.p >= 1
            && self.p <= MAX_P
            && self.t >= 1
            && self.t <= MAX_T
            && self.m_kib >= self.p.saturating_mul(8)
            && self.m_kib <= MAX_M_KIB;
        if ok { Ok(()) } else { Err(VaultError::Corrupt) }
    }
}

/// Подбирает `t`, чтобы одно вычисление KEK занимало 300–700 мс.
///
/// Если уже `t = 3` даёт ≥ 250 мс, остаются параметры по умолчанию.
pub fn calibrate() -> Result<KdfParams, VaultError> {
    let base = KdfParams::DEFAULT;
    let started = Instant::now();
    derive_kek(b"calibration", &[0u8; SALT_LEN], base)?;
    let elapsed = started.elapsed();
    if elapsed >= CALIBRATION_FLOOR {
        return Ok(base);
    }
    let per_pass = (elapsed.as_micros() / u128::from(base.t)).max(1);
    let t = (CALIBRATION_TARGET_MICROS / per_pass).clamp(u128::from(base.t), u128::from(MAX_T));
    Ok(KdfParams {
        t: u32::try_from(t).unwrap_or(MAX_T),
        ..base
    })
}

pub(crate) fn derive_kek(
    secret: &[u8],
    salt: &[u8],
    params: KdfParams,
) -> Result<Zeroizing<[u8; KEY_LEN]>, VaultError> {
    let argon_params = Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
        .map_err(|_| VaultError::Kdf)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut kek = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(secret, salt, kek.as_mut_slice())
        .map_err(|_| VaultError::Kdf)?;
    Ok(kek)
}

pub(crate) fn fill_random(buf: &mut [u8]) -> Result<(), VaultError> {
    getrandom::fill(buf).map_err(|_| VaultError::Random)
}

pub(crate) fn random_array<const N: usize>() -> Result<[u8; N], VaultError> {
    let mut buf = [0u8; N];
    fill_random(&mut buf)?;
    Ok(buf)
}

/// DEK, обёрнутый ключом, выведенным из пароля или recovery-кода.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct KeySlot {
    #[serde(with = "b64")]
    salt: Vec<u8>,
    #[serde(with = "b64")]
    nonce: Vec<u8>,
    #[serde(with = "b64")]
    wrapped_dek: Vec<u8>,
}

fn aad(vault_id: &str) -> Vec<u8> {
    format!("{AAD_PREFIX}{vault_id}").into_bytes()
}

impl KeySlot {
    /// Оборачивает `dek` ключом из `secret` с новыми salt и nonce.
    pub(crate) fn wrap(
        secret: &[u8],
        vault_id: &str,
        params: KdfParams,
        dek: &Dek,
    ) -> Result<Self, VaultError> {
        let salt = random_array::<SALT_LEN>()?;
        let nonce = random_array::<NONCE_LEN>()?;
        let kek = derive_kek(secret, &salt, params)?;
        let cipher =
            XChaCha20Poly1305::new_from_slice(kek.as_slice()).map_err(|_| VaultError::Crypto)?;
        let xnonce = XNonce::try_from(nonce.as_slice()).map_err(|_| VaultError::Crypto)?;
        let wrapped_dek = cipher
            .encrypt(
                &xnonce,
                Payload {
                    msg: dek.as_bytes(),
                    aad: &aad(vault_id),
                },
            )
            .map_err(|_| VaultError::Crypto)?;
        Ok(Self {
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            wrapped_dek,
        })
    }

    /// Разворачивает DEK. Ошибка AEAD → `on_fail` (неверный пароль или код):
    /// отдельный «хеш пароля» не храним.
    pub(crate) fn unwrap(
        &self,
        secret: &[u8],
        vault_id: &str,
        params: KdfParams,
        on_fail: VaultError,
    ) -> Result<Dek, VaultError> {
        let kek = derive_kek(secret, &self.salt, params)?;
        let cipher =
            XChaCha20Poly1305::new_from_slice(kek.as_slice()).map_err(|_| VaultError::Crypto)?;
        let xnonce = XNonce::try_from(self.nonce.as_slice()).map_err(|_| VaultError::Corrupt)?;
        let plain = cipher
            .decrypt(
                &xnonce,
                Payload {
                    msg: &self.wrapped_dek,
                    aad: &aad(vault_id),
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| on_fail)?;
        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        if plain.len() != KEY_LEN {
            return Err(VaultError::Corrupt);
        }
        key.copy_from_slice(&plain);
        Ok(Dek(key))
    }

    pub(crate) fn validate(&self) -> Result<(), VaultError> {
        if self.salt.len() == SALT_LEN
            && self.nonce.len() == NONCE_LEN
            && !self.wrapped_dek.is_empty()
        {
            Ok(())
        } else {
            Err(VaultError::Corrupt)
        }
    }
}

mod b64 {
    use super::{BASE64, Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&BASE64.encode(bytes))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        BASE64
            .decode(text.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

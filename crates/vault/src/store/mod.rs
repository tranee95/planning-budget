//! Операции над сейфом: создание, вход, смена пароля, восстановление.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};

use crate::crypto::{self, Dek, KdfParams, KeySlot, random_array};
use crate::error::VaultError;
use crate::file::{VaultDir, VaultFile};
use crate::recovery::{self, RecoveryCode};

/// Минимальная длина пароля в символах.
pub const MIN_PASSWORD_CHARS: usize = 10;

mod backoff;
mod rekey;
mod restore;

pub use backoff::backoff_secs;
use backoff::retry_after;

/// Результат `create`: DEK для открытия БД и recovery-код, который
/// показывается пользователю один раз.
#[derive(Debug)]
pub struct Created {
    pub dek: Dek,
    pub recovery_code: RecoveryCode,
}

/// Начатый перевыпуск ключа: новый DEK для `PRAGMA rekey` и новый recovery-код.
#[derive(Debug)]
pub struct RekeyPlan {
    pub new_dek: Dek,
    pub recovery_code: RecoveryCode,
}

/// Что известно о сейфе до разблокировки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultStatus {
    pub exists: bool,
    pub retry_after_secs: u64,
}

/// Сейф в каталоге данных приложения.
#[derive(Debug, Clone)]
pub struct VaultStore {
    dir: VaultDir,
}

impl VaultStore {
    #[must_use]
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            dir: VaultDir::new(data_dir),
        }
    }

    pub fn status(&self, now: DateTime<Utc>) -> Result<VaultStatus, VaultError> {
        if !self.dir.exists() {
            return Ok(VaultStatus {
                exists: false,
                retry_after_secs: 0,
            });
        }
        let file = self.dir.load()?;
        Ok(VaultStatus {
            exists: true,
            retry_after_secs: retry_after(&file, now),
        })
    }

    /// Создаёт сейф с калиброванными параметрами Argon2id.
    pub fn create(
        &self,
        password: &SecretString,
        now: DateTime<Utc>,
    ) -> Result<Created, VaultError> {
        self.create_with_params(password, crypto::calibrate()?, now)
    }

    /// То же с явными параметрами KDF (тесты используют облегчённые).
    pub fn create_with_params(
        &self,
        password: &SecretString,
        params: KdfParams,
        now: DateTime<Utc>,
    ) -> Result<Created, VaultError> {
        check_new_password(password)?;
        params.validate()?;
        if self.dir.exists() {
            return Err(VaultError::AlreadyExists);
        }
        let vault_id = new_vault_id()?;
        let dek = Dek::random()?;
        let recovery_code = RecoveryCode::generate()?;
        let pw = KeySlot::wrap(password.expose_secret().as_bytes(), &vault_id, params, &dek)?;
        let rc = KeySlot::wrap(recovery_code.secret(), &vault_id, params, &dek)?;
        self.dir
            .save(&VaultFile::new(vault_id, params, pw, rc, now))?;
        Ok(Created { dek, recovery_code })
    }

    /// Вход по паролю. Неверный пароль увеличивает счётчик в `vault.json`,
    /// после третьей подряд неудачи включается задержка.
    pub fn unlock(&self, password: &SecretString, now: DateTime<Utc>) -> Result<Dek, VaultError> {
        let (_, dek) = self.verify_password(password, now)?;
        Ok(dek)
    }

    /// Смена пароля: DEK не меняется, перезаписывается только слот `pw`.
    pub fn change_password(
        &self,
        old: &SecretString,
        new: &SecretString,
        now: DateTime<Utc>,
    ) -> Result<(), VaultError> {
        check_new_password(new)?;
        let (mut file, dek) = self.verify_password(old, now)?;
        let (vault_id, params) = (file.vault_id.clone(), file.params());
        if let Some(next) = &mut file.next {
            // Недоведённый перевыпуск: новый DEK тоже должен открываться новым паролем,
            // иначе после смены пароля он останется под старым и станет недоступен.
            let next_dek = next.pw.unwrap(
                old.expose_secret().as_bytes(),
                &vault_id,
                params,
                VaultError::Corrupt,
            )?;
            next.pw = KeySlot::wrap(new.expose_secret().as_bytes(), &vault_id, params, &next_dek)?;
        }
        file.pw = KeySlot::wrap(
            new.expose_secret().as_bytes(),
            &file.vault_id,
            file.params(),
            &dek,
        )?;
        self.dir.save(&file)
    }

    /// Удаляет `vault.json`: после этого данные не открыть ни паролем, ни recovery-кодом.
    /// Вызывающий отвечает за подтверждение пользователя (`vault_reset`).
    pub fn destroy(&self) -> Result<(), VaultError> {
        self.dir.destroy()
    }

    fn verify_password(
        &self,
        password: &SecretString,
        now: DateTime<Utc>,
    ) -> Result<(VaultFile, Dek), VaultError> {
        let mut file = self.dir.load_for_update()?;
        if file.last_failed_at.is_some_and(|last| last > now) {
            // Последняя неудача «в будущем»: часы перевели назад. Без поправки задержка
            // длилась бы, пока часы не догонят старую отметку, — хоть год.
            file.last_failed_at = Some(now);
            self.dir.save(&file)?;
        }
        let wait = retry_after(&file, now);
        if wait > 0 {
            return Err(VaultError::Backoff {
                retry_after_secs: wait,
            });
        }
        let unwrapped = file.pw.unwrap(
            password.expose_secret().as_bytes(),
            &file.vault_id,
            file.params(),
            VaultError::WrongPassword,
        );
        match unwrapped {
            Ok(dek) => {
                if file.failed_attempts > 0 {
                    file.failed_attempts = 0;
                    file.last_failed_at = None;
                    self.dir.save(&file)?;
                }
                Ok((file, dek))
            }
            Err(VaultError::WrongPassword) => {
                file.failed_attempts = file.failed_attempts.saturating_add(1);
                file.last_failed_at = Some(now);
                self.dir.save(&file)?;
                Err(VaultError::WrongPassword)
            }
            Err(e) => Err(e),
        }
    }
}

fn unwrap_with_recovery(file: &VaultFile, code: &SecretString) -> Result<Dek, VaultError> {
    let secret = recovery::parse(code)?;
    file.rc.unwrap(
        &secret,
        &file.vault_id,
        file.params(),
        VaultError::WrongRecoveryCode,
    )
}

fn check_new_password(password: &SecretString) -> Result<(), VaultError> {
    if password.expose_secret().chars().count() < MIN_PASSWORD_CHARS {
        return Err(VaultError::PasswordTooShort {
            min: MIN_PASSWORD_CHARS,
        });
    }
    Ok(())
}

/// UUID v4 из случайных байтов: отдельная зависимость ради одного поля не нужна.
fn new_vault_id() -> Result<String, VaultError> {
    let mut bytes = random_array::<16>()?;
    for (idx, mask, tag) in [(6, 0x0f, 0x40), (8, 0x3f, 0x80)] {
        if let Some(b) = bytes.get_mut(idx) {
            *b = (*b & mask) | tag;
        }
    }
    let hex = data_encoding::HEXLOWER.encode(&bytes);
    let part = |range: std::ops::Range<usize>| hex.get(range).unwrap_or_default().to_owned();
    Ok([
        part(0..8),
        part(8..12),
        part(12..16),
        part(16..20),
        part(20..32),
    ]
    .join("-"))
}

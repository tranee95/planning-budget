//! Перевыпуск ключа шифрования: шаги 1–3 и откат.

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};

use super::{RekeyPlan, VaultStore, unwrap_with_recovery};
use crate::crypto::{Dek, KeySlot};
use crate::error::VaultError;
use crate::file::NextKeys;
use crate::recovery::RecoveryCode;

impl VaultStore {
    /// Перевыпуск ключа, шаг 1: проверяет пароль, создаёт новый DEK и новый recovery-код
    /// и записывает их слоты в `vault.json` рядом со старыми (блок `next`).
    /// Старый пароль и код продолжают работать, пока шаг 3 не заменит слоты.
    pub fn begin_rekey(
        &self,
        password: &SecretString,
        now: DateTime<Utc>,
    ) -> Result<RekeyPlan, VaultError> {
        let (mut file, old_dek) = self.verify_password(password, now)?;
        if file.next.is_some() {
            // Слоты прошлого перевыпуска могут быть единственной копией ключа, под которым
            // уже лежит база. Перезаписывать их нельзя: сначала вход доводит дело до конца.
            return Err(VaultError::RekeyPending);
        }
        let new_dek = Dek::random()?;
        let recovery_code = RecoveryCode::generate()?;
        let pw = KeySlot::wrap(
            password.expose_secret().as_bytes(),
            &file.vault_id,
            file.params(),
            &new_dek,
        )?;
        let rc = KeySlot::wrap(
            recovery_code.secret(),
            &file.vault_id,
            file.params(),
            &new_dek,
        )?;
        let via_old = KeySlot::wrap(old_dek.as_bytes(), &file.vault_id, file.params(), &new_dek)?;
        file.next = Some(NextKeys {
            pw,
            rc,
            via_old: Some(via_old),
        });
        self.dir.save(&file)?;
        Ok(RekeyPlan {
            new_dek,
            recovery_code,
        })
    }

    /// Шаг 3, после успешного `rekey` базы: новые слоты становятся основными,
    /// старые стираются — старый пароль и старый код больше ничего не разворачивают.
    pub fn finish_rekey(&self) -> Result<(), VaultError> {
        let mut file = self.dir.load_for_update()?;
        if let Some(next) = file.next.take() {
            file.pw = next.pw;
            file.rc = next.rc;
            self.dir.save(&file)?;
        }
        Ok(())
    }

    /// Шаг 3 для прерванного перевыпуска: новый recovery-код пользователю не показывали,
    /// поэтому вместо недоступного `next.rc` выпускается свежий код под `new_dek`
    /// (ключ, под которым уже лежит база). Старый код после этого не работает.
    pub fn finish_rekey_with_new_code(&self, new_dek: &Dek) -> Result<RecoveryCode, VaultError> {
        let mut file = self.dir.load_for_update()?;
        let Some(next) = file.next.take() else {
            return Err(VaultError::Corrupt);
        };
        let code = RecoveryCode::generate()?;
        file.pw = next.pw;
        file.rc = KeySlot::wrap(code.secret(), &file.vault_id, file.params(), new_dek)?;
        self.dir.save(&file)?;
        Ok(code)
    }

    /// Откат шага 1, если база не перешифровалась.
    pub fn abort_rekey(&self) -> Result<(), VaultError> {
        let mut file = self.dir.load_for_update()?;
        if file.next.take().is_some() {
            self.dir.save(&file)?;
        }
        Ok(())
    }

    /// Есть ли недоведённый перевыпуск (процесс прервался после шага 1).
    pub fn rekey_pending(&self) -> Result<bool, VaultError> {
        Ok(self.dir.load()?.next.is_some())
    }

    /// Новый DEK из недоведённого перевыпуска. Нужен, когда база уже под новым ключом,
    /// а основные слоты ещё старые. Счётчик неудачных попыток не меняется.
    pub fn pending_rekey_dek(&self, password: &SecretString) -> Result<Option<Dek>, VaultError> {
        let file = self.dir.load_for_update()?;
        let Some(next) = &file.next else {
            return Ok(None);
        };
        next.pw
            .unwrap(
                password.expose_secret().as_bytes(),
                &file.vault_id,
                file.params(),
                VaultError::WrongPassword,
            )
            .map(Some)
    }

    /// Новый DEK недоведённого перевыпуска, полученный из старого recovery-кода:
    /// код открывает старый DEK, старый DEK — новый. `None`, если перевыпуска нет или файл
    /// записан до появления цепочки.
    pub fn pending_rekey_dek_via_recovery(
        &self,
        code: &SecretString,
    ) -> Result<Option<Dek>, VaultError> {
        let file = self.dir.load_for_update()?;
        let Some(via_old) = file.next.as_ref().and_then(|n| n.via_old.as_ref()) else {
            return Ok(None);
        };
        let old = unwrap_with_recovery(&file, code)?;
        via_old
            .unwrap(
                old.as_bytes(),
                &file.vault_id,
                file.params(),
                VaultError::Corrupt,
            )
            .map(Some)
    }
}

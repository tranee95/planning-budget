//! Вход по recovery-коду и замена забытого пароля.

use secrecy::{ExposeSecret, SecretString};

use super::{VaultStore, check_new_password, unwrap_with_recovery};
use crate::crypto::{Dek, KeySlot};
use crate::error::VaultError;

impl VaultStore {
    /// Вход по recovery-коду. Код на 128 бит перебору не поддаётся,
    /// поэтому счётчик попыток не ведётся.
    pub fn unlock_recovery(&self, code: &SecretString) -> Result<Dek, VaultError> {
        let file = self.dir.load_for_update()?;
        unwrap_with_recovery(&file, code)
    }

    /// «Забыл пароль»: recovery-код открывает DEK, слот `pw` пишется заново
    /// под новый пароль, счётчик неудач сбрасывается.
    pub fn reset_password_with_recovery(
        &self,
        code: &SecretString,
        new: &SecretString,
    ) -> Result<Dek, VaultError> {
        check_new_password(new)?;
        let mut file = self.dir.load_for_update()?;
        let dek = unwrap_with_recovery(&file, code)?;
        let (vault_id, params) = (file.vault_id.clone(), file.params());
        if let Some(next) = &mut file.next
            && let Some(via_old) = &next.via_old
        {
            // Недоведённый перевыпуск: новый DEK тоже переезжает под новый пароль,
            // иначе вход по новому паролю не найдёт перешифрованную базу.
            let next_dek =
                via_old.unwrap(dek.as_bytes(), &vault_id, params, VaultError::Corrupt)?;
            next.pw = KeySlot::wrap(new.expose_secret().as_bytes(), &vault_id, params, &next_dek)?;
        }
        file.pw = KeySlot::wrap(new.expose_secret().as_bytes(), &vault_id, params, &dek)?;
        file.failed_attempts = 0;
        file.last_failed_at = None;
        self.dir.save(&file)?;
        Ok(dek)
    }
}

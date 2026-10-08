//! Перевыпуск ключа и доведение прерванного перевыпуска при входе.

use chrono::{DateTime, Utc};
use planning_budget_storage::{Db, StorageError};
use planning_budget_vault::{Dek, RecoveryCode, VaultStore};
use secrecy::SecretString;
use zeroize::Zeroizing;

use super::{finish_unlock, vault_err};
use crate::AppError;
use crate::state::AppState;

/// Открывает сессию и только после этого доводит перевыпуск до конца: новый recovery-код
/// записывается в `vault.json` (старый перестаёт работать), когда вход уже удался.
/// Если сессию открыть не вышло, слоты не тронуты, и следующий вход повторит доведение.
pub(super) fn complete_unlock(
    state: &AppState,
    store: &VaultStore,
    db: Db,
    pending: Option<Dek>,
    now: DateTime<Utc>,
) -> Result<Option<Zeroizing<String>>, AppError> {
    finish_unlock(state, db)?;
    let Some(next) = pending else {
        return Ok(None);
    };
    match store.finish_rekey_with_new_code(&next) {
        Ok(code) => {
            tracing::warn!("interrupted key reissue completed; a new recovery code was issued");
            Ok(issued(state, Some(code)))
        }
        Err(err) => {
            state.lock();
            Err(vault_err(store, now, err))
        }
    }
}

/// Выданный при входе код можно сохранить в файл так же, как после перевыпуска.
fn issued(state: &AppState, code: Option<RecoveryCode>) -> Option<Zeroizing<String>> {
    let code = code?;
    state.allow_recovery_save(true);
    Some(code.display())
}

/// Открывает БД при входе. Если прошлый перевыпуск ключа прервался, решает по тому,
/// каким ключом читается база:
/// - старым — `rekey` не дошёл до файла, недоведённые слоты отбрасываются;
/// - новым — база уже перешифрована; ключ `next` возвращается вызывающему, и `complete_unlock`
///   доводит слоты до конца уже после успешного входа, выдавая новый recovery-код:
///   показанный при перевыпуске пользователь не видел.
///
/// `password` — пароль, под которым сейчас лежит слот `next`. При входе по recovery-коду
/// `reset_password_with_recovery` уже переносит `next` под новый пароль, так что
/// сюда приходит новый пароль.
pub(super) fn open_db_finishing_rekey(
    state: &AppState,
    store: &VaultStore,
    password: &SecretString,
    dek: &Dek,
    now: DateTime<Utc>,
) -> Result<(Db, Option<Dek>), AppError> {
    let path = state.paths().db();
    if !store
        .rekey_pending()
        .map_err(|e| vault_err(store, now, e))?
    {
        return Ok((Db::open(&path, dek.as_bytes())?, None));
    }
    match Db::open(&path, dek.as_bytes()) {
        Ok(db) => {
            store.abort_rekey().map_err(|e| vault_err(store, now, e))?;
            tracing::warn!("interrupted key reissue discarded: database kept its key");
            Ok((db, None))
        }
        Err(StorageError::KeyRejected) => {
            let interrupted = || AppError::Conflict {
                message_key: "errors.vault.rekey_interrupted".into(),
            };
            let next = store
                .pending_rekey_dek(password)
                .map_err(|_| interrupted())?
                .ok_or_else(interrupted)?;
            let db = Db::open(&path, next.as_bytes())?;
            Ok((db, Some(next)))
        }
        Err(e) => Err(e.into()),
    }
}

/// Перевыпуск ключа шифрования: новый DEK, `PRAGMA rekey`, новый recovery-код.
///
/// Порядок защищает от потери данных при сбое: сначала новые слоты пишутся рядом со старыми,
/// потом перешифровывается база, и только после этого старые слоты стираются.
///
/// При любой ошибке после записи новых слотов они **не удаляются**: возможно, это уже
/// единственная копия ключа, под которым лежит база. Сессия закрывается, и следующий вход
/// определяет ключ по самой базе (`open_db_finishing_rekey`).
pub fn rekey(
    state: &AppState,
    password: &SecretString,
    now: DateTime<Utc>,
) -> Result<Zeroizing<String>, AppError> {
    if !state.is_unlocked() {
        return Err(AppError::Locked);
    }
    let store = state.vault();
    let plan = store
        .begin_rekey(password, now)
        .map_err(|e| vault_err(&store, now, e))?;
    let key = Zeroizing::new(*plan.new_dek.as_bytes());
    if let Err(err) = state.with_session_sync(|db| db.rekey(&key).map_err(AppError::from)) {
        state.lock();
        return Err(err);
    }
    if let Err(err) = store.finish_rekey() {
        state.lock();
        return Err(vault_err(&store, now, err));
    }
    state.allow_recovery_save(true);
    Ok(plan.recovery_code.display())
}

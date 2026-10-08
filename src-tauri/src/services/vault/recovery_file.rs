//! Сохранение recovery-кода в файл, выбранный пользователем.

use std::fs;

use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::AppError;
use crate::state::AppState;

/// Пишет recovery-код в файл, выбранный пользователем в нативном диалоге.
/// Разрешено только сразу после выдачи кода (`create`, `rekey`).
pub fn save_recovery_code(
    state: &AppState,
    code: &SecretString,
    pick_path: impl FnOnce() -> Option<std::path::PathBuf>,
) -> Result<bool, AppError> {
    use secrecy::ExposeSecret as _;

    if !state.recovery_save_allowed() {
        return Err(AppError::Conflict {
            message_key: "errors.vault.recovery_save_not_allowed".into(),
        });
    }
    let Some(path) = pick_path() else {
        return Ok(false);
    };
    let text = Zeroizing::new(format!("{}\n", code.expose_secret()));
    write_private_file(&path, text.as_bytes()).map_err(|_| AppError::Io {
        message_key: "errors.io.recovery_file".into(),
    })?;
    state.allow_recovery_save(false);
    Ok(true)
}

/// Пишет файл, читаемый только владельцем (`0600` на unix). Режим при создании не
/// трогает уже существующий файл, поэтому права выставляются ещё раз до записи секрета.
fn write_private_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
        options.mode(0o600);
        let mut file = options.open(path)?;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        file.write_all(bytes)
    }
    #[cfg(not(unix))]
    {
        options.open(path)?.write_all(bytes)
    }
}

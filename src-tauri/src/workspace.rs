//! `workspace.json`: какие хранилища есть, какое активно и куда писать резервные копии
//!. Читается до разблокировки. Данных бюджета в файле нет; названия
//! хранилищ задаёт пользователь, в журнал они не попадают.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::AppError;
use crate::sync::MutexExt as _;

/// Хранилище, созданное до появления выбора: лежит прямо в каталоге данных.
pub const MAIN_ID: &str = "main";
pub const MAX_LABEL_CHARS: usize = 60;
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    version: u32,
    pub active: String,
    /// Дополнительные хранилища; основное (`main`) в списке не хранится.
    #[serde(default)]
    pub vaults: Vec<VaultEntry>,
    /// Каталог резервных копий, выбранный пользователем; `None` — `backups/` в каталоге данных.
    #[serde(default)]
    pub backup_dir: Option<PathBuf>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            version: VERSION,
            active: MAIN_ID.to_owned(),
            vaults: Vec::new(),
            backup_dir: None,
        }
    }
}

/// Идентификатор попадает в имя папки и файла: только hex и дефисы, чтобы из него нельзя было
/// собрать путь вне `vaults/`.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    id == MAIN_ID
        || (!id.is_empty()
            && id.len() <= 36
            && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
}

/// Название: непустое после обрезки, без управляющих символов, не длиннее [`MAX_LABEL_CHARS`].
///
/// # Errors
/// `Validation` с полем `label`.
pub fn clean_label(raw: &str) -> Result<String, AppError> {
    let label = raw.trim();
    if label.is_empty()
        || label.chars().count() > MAX_LABEL_CHARS
        || label.chars().any(char::is_control)
    {
        return Err(AppError::invalid_field("label_invalid", "label"));
    }
    Ok(label.to_owned())
}

impl Workspace {
    /// Папка хранилища. Не проверяет, что оно зарегистрировано.
    #[must_use]
    pub fn vault_dir(data_dir: &Path, id: &str) -> PathBuf {
        if id == MAIN_ID {
            data_dir.to_path_buf()
        } else {
            data_dir.join("vaults").join(id)
        }
    }

    #[must_use]
    pub fn knows(&self, id: &str) -> bool {
        id == MAIN_ID || self.vaults.iter().any(|v| v.id == id)
    }

    /// Активное хранилище, которого нет в списке (файл правили руками), заменяется основным.
    fn repaired(mut self) -> Self {
        if !self.knows(&self.active) {
            self.active = MAIN_ID.to_owned();
        }
        self.vaults.retain(|v| valid_id(&v.id) && v.id != MAIN_ID);
        self
    }
}

/// Чтение-изменение-запись файла идут по одному: две команды подряд не затирают друг друга.
static WRITE: Mutex<()> = Mutex::new(());

/// Нет файла или он повреждён — одно основное хранилище: из-за списка приложение не должно
/// отказываться стартовать, а данные основного хранилища лежат на прежнем месте.
#[must_use]
pub fn load(path: &Path) -> Workspace {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Workspace>(&bytes).ok())
        .map(Workspace::repaired)
        .unwrap_or_default()
}

fn save(path: &Path, workspace: &Workspace) -> Result<(), AppError> {
    let io = || AppError::Io {
        message_key: "errors.io.workspace".into(),
    };
    let bytes =
        serde_json::to_vec_pretty(workspace).map_err(|e| AppError::internal("workspace", &e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(|_| io())?;
    fs::rename(&tmp, path).map_err(|_| io())
}

/// Читает файл, применяет `change` и сохраняет результат (если `change` вернул `Ok`).
///
/// # Errors
/// Ошибка из `change` или `Io` при записи.
pub fn update<T>(
    path: &Path,
    change: impl FnOnce(&mut Workspace) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let _guard = WRITE.lock_unpoisoned();
    let mut workspace = load(path);
    let out = change(&mut workspace)?;
    save(path, &workspace)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("workspace.json")
    }

    #[test]
    fn missing_or_broken_file_means_the_main_vault_only() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&file(&dir)), Workspace::default());
        fs::write(file(&dir), b"{ not json").unwrap();
        assert_eq!(load(&file(&dir)).active, MAIN_ID);
    }

    #[test]
    fn update_round_trips_and_unknown_active_falls_back_to_main() {
        let dir = tempfile::tempdir().unwrap();
        update(&file(&dir), |w| {
            w.vaults.push(VaultEntry {
                id: "abc-123".into(),
                label: "Семья".into(),
            });
            w.active = "abc-123".into();
            Ok(())
        })
        .unwrap();
        assert_eq!(load(&file(&dir)).active, "abc-123");
        update(&file(&dir), |w| {
            w.vaults.clear();
            Ok(())
        })
        .unwrap();
        assert_eq!(load(&file(&dir)).active, MAIN_ID);
    }

    #[test]
    fn failed_change_leaves_the_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        update(&file(&dir), |w| {
            w.backup_dir = Some(PathBuf::from("x"));
            Ok(())
        })
        .unwrap();
        let failed: Result<(), AppError> = update(&file(&dir), |w| {
            w.backup_dir = None;
            Err(AppError::Conflict {
                message_key: "errors.test".into(),
            })
        });
        assert!(failed.is_err());
        assert!(load(&file(&dir)).backup_dir.is_some());
    }

    #[test]
    fn ids_cannot_escape_the_vaults_folder() {
        for bad in ["", "..", "a/b", "a\\b", "C:", "x y", &"a".repeat(37)] {
            assert!(!valid_id(bad), "{bad}");
        }
        assert!(valid_id("main") && valid_id("0f3a-9b"));
    }

    #[test]
    fn labels_are_trimmed_and_bounded() {
        assert_eq!(clean_label("  Семья ").unwrap(), "Семья");
        assert!(clean_label("   ").is_err());
        assert!(clean_label("a\nb").is_err());
        assert!(clean_label(&"я".repeat(61)).is_err());
    }
}

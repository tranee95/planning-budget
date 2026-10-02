//! `vault.json`: формат и атомарная запись.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::crypto::{KdfParams, KeySlot};
use crate::error::VaultError;

pub(crate) const VAULT_FILE: &str = "vault.json";
const TMP_FILE: &str = "vault.json.tmp";
const BAK_FILE: &str = "vault.json.bak";
const FORMAT_VERSION: u32 = 1;
const KDF_ALG: &str = "argon2id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct KdfRecord {
    alg: String,
    #[serde(flatten)]
    params: KdfParams,
}

/// Слоты нового DEK на время перевыпуска ключа. Пока блок есть, старые слоты
/// остаются рабочими: если процесс прервался между `rekey` базы и записью
/// итогового файла, следующий вход находит базу под новым ключом и доводит дело до конца.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NextKeys {
    pub(crate) pw: KeySlot,
    pub(crate) rc: KeySlot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VaultFile {
    version: u32,
    pub(crate) vault_id: String,
    kdf: KdfRecord,
    pub(crate) pw: KeySlot,
    pub(crate) rc: KeySlot,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) failed_attempts: u32,
    pub(crate) last_failed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) next: Option<NextKeys>,
}

impl VaultFile {
    pub(crate) fn new(
        vault_id: String,
        params: KdfParams,
        pw: KeySlot,
        rc: KeySlot,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            version: FORMAT_VERSION,
            vault_id,
            kdf: KdfRecord {
                alg: KDF_ALG.to_owned(),
                params,
            },
            pw,
            rc,
            created_at,
            failed_attempts: 0,
            last_failed_at: None,
            next: None,
        }
    }

    pub(crate) fn params(&self) -> KdfParams {
        self.kdf.params
    }

    fn validate(&self) -> Result<(), VaultError> {
        if self.version != FORMAT_VERSION {
            return Err(VaultError::UnsupportedVersion(self.version));
        }
        if self.kdf.alg != KDF_ALG || self.vault_id.is_empty() {
            return Err(VaultError::Corrupt);
        }
        self.kdf.params.validate()?;
        self.pw.validate()?;
        self.rc.validate()?;
        match &self.next {
            Some(next) => next.pw.validate().and_then(|()| next.rc.validate()),
            None => Ok(()),
        }
    }
}

/// Каталог с `vault.json` и его временными соседями.
#[derive(Debug, Clone)]
pub(crate) struct VaultDir {
    root: PathBuf,
}

impl VaultDir {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub(crate) fn exists(&self) -> bool {
        self.path(VAULT_FILE).exists() || self.path(BAK_FILE).exists()
    }

    /// Затирает и удаляет `vault.json` вместе с `.bak` и `.tmp`, если они есть.
    pub(crate) fn destroy(&self) -> Result<(), VaultError> {
        for name in [VAULT_FILE, BAK_FILE] {
            let path = self.path(name);
            if path.exists() {
                shred(&path)?;
            }
        }
        match fs::remove_file(self.path(TMP_FILE)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// Читает `vault.json`, ничего не меняя на диске. Если основной файл потерян или
    /// повреждён прерванной записью, берётся `.bak`.
    ///
    /// Чтение без побочных эффектов нужно `vault_status`: он вызывается в любой момент
    /// и не должен трогать файлы, которые в эту секунду пишет другая операция.
    pub(crate) fn load(&self) -> Result<VaultFile, VaultError> {
        match read_file(&self.path(VAULT_FILE)) {
            Ok(file) => Ok(file),
            Err(main_err) => match read_file(&self.path(BAK_FILE)) {
                Ok(file) => Ok(file),
                Err(_) if matches!(main_err, VaultError::Io(std::io::ErrorKind::NotFound)) => {
                    Err(VaultError::NotFound)
                }
                Err(_) => Err(main_err),
            },
        }
    }

    /// Добивает прерванную запись и читает файл: остаток `.tmp` удаляется, а `.bak` либо
    /// уничтожается (основной файл цел), либо возвращается на место. Вызывается в начале
    /// каждой изменяющей операции; вызывающий отвечает за то, что они не идут параллельно.
    pub(crate) fn load_for_update(&self) -> Result<VaultFile, VaultError> {
        let tmp = self.path(TMP_FILE);
        if tmp.exists() {
            fs::remove_file(&tmp)?;
        }
        let main = self.path(VAULT_FILE);
        let bak = self.path(BAK_FILE);
        if bak.exists() {
            if read_file(&main).is_ok() {
                shred(&bak)?;
            } else if read_file(&bak).is_ok() {
                fs::rename(&bak, &main)?;
            }
        }
        self.load()
    }

    /// `vault.json.tmp` → `fsync` → `rename`. Предыдущая версия лежит в `.bak`
    /// только на время записи и затирается сразу после проверки новой:
    /// иначе старый пароль продолжал бы разворачивать DEK.
    pub(crate) fn save(&self, file: &VaultFile) -> Result<(), VaultError> {
        let main = self.path(VAULT_FILE);
        let tmp = self.path(TMP_FILE);
        let bak = self.path(BAK_FILE);
        let had_previous = main.exists();
        if had_previous {
            fs::copy(&main, &bak)?;
        }

        let bytes = serde_json::to_vec_pretty(file).map_err(|_| VaultError::Corrupt)?;
        let written = write_synced(&tmp, &bytes)
            .and_then(|()| fs::rename(&tmp, &main).map_err(VaultError::from))
            .and_then(|()| sync_dir(&self.root))
            .and_then(|()| match read_file(&main) {
                Ok(back) if back == *file => Ok(()),
                _ => Err(VaultError::Corrupt),
            });

        match written {
            Ok(()) => {
                if had_previous {
                    shred(&bak)?;
                }
                Ok(())
            }
            Err(e) => {
                // Возвращаем прежнюю версию, только если она читается: иначе лучше оставить
                // то, что есть, чем подменить файл ключей мусором.
                if had_previous && read_file(&bak).is_ok() {
                    let _ = fs::rename(&bak, &main);
                }
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }
}

fn read_file(path: &Path) -> Result<VaultFile, VaultError> {
    let bytes = fs::read(path)?;
    let file: VaultFile = serde_json::from_slice(&bytes).map_err(|_| VaultError::Corrupt)?;
    file.validate()?;
    Ok(file)
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), VaultError> {
    let mut f = File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<(), VaultError> {
    File::open(dir)?.sync_all()?;
    Ok(())
}

/// На Windows каталог нельзя открыть как файл; `rename` там уже синхронный.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps, reason = "общая сигнатура с unix-вариантом")]
fn sync_dir(_dir: &Path) -> Result<(), VaultError> {
    Ok(())
}

/// Перезапись нулями и удаление.
fn shred(path: &Path) -> Result<(), VaultError> {
    let len = fs::metadata(path)?.len();
    {
        let mut f = OpenOptions::new().write(true).open(path)?;
        let zeros = [0u8; 4096];
        let mut left = len;
        while left > 0 {
            let chunk = usize::try_from(left.min(zeros.len() as u64)).unwrap_or(zeros.len());
            f.write_all(zeros.get(..chunk).unwrap_or(&zeros))?;
            left = left.saturating_sub(chunk as u64);
        }
        f.sync_all()?;
    }
    fs::remove_file(path)?;
    Ok(())
}

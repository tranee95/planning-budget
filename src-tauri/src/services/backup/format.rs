//! Файл резервной копии `.budgetbak`.
//!
//! ```text
//! "PBBAK\x01\0\0"            8 байт: признак и версия формата
//! u32 LE n, manifest.json    n ≤ 4 КиБ: формат, версия приложения, vault_id, время
//! u32 LE n, vault.json       n ≤ 64 КиБ: обёрнутые ключи (зашифрованы паролем и recovery-кодом)
//! u64 LE n, budget.db        база целиком (зашифрована SQLCipher)
//! ```
//!
//! Это не zip: внутри нет имён файлов и путей, поэтому из чужой копии нельзя записать файл
//! вне целевой папки, а лишняя зависимость для чтения чужих архивов не нужна. Данных в открытом
//! виде в файле нет: в манифесте только идентификатор сейфа, он и так лежит в `vault.json`.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::AppError;

const MAGIC: [u8; 8] = *b"PBBAK\x01\0\0";
const MAX_MANIFEST: u32 = 4 * 1024;
const MAX_VAULT: u32 = 64 * 1024;
const FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub app_version: String,
    pub vault_id: String,
    pub created_at: String,
}

impl Manifest {
    #[must_use]
    pub fn new(vault_id: String, created_at: String) -> Self {
        Self {
            format: FORMAT,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            vault_id,
            created_at,
        }
    }
}

fn io_err(key: &str) -> AppError {
    AppError::Io {
        message_key: format!("errors.io.{key}"),
    }
}

fn bad_file() -> AppError {
    AppError::Validation {
        message_key: "errors.backup.bad_file".into(),
        field: None,
    }
}

/// Пишет копию во временный файл рядом и переименовывает: оборванная запись не оставляет
/// обрезанный `.budgetbak`, который выглядел бы настоящим. Временный файл зашифрован так же.
///
/// # Errors
/// `Io` при любой ошибке записи (временный файл удаляется).
pub fn write(
    dest: &Path,
    manifest: &Manifest,
    vault_json: &[u8],
    db: &Path,
) -> Result<(), AppError> {
    let tmp = dest.with_extension("budgetbak.tmp");
    let result = write_to(&tmp, manifest, vault_json, db)
        .and_then(|()| fs::rename(&tmp, dest).map_err(|_| io_err("backup_write")));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_to(tmp: &Path, manifest: &Manifest, vault_json: &[u8], db: &Path) -> Result<(), AppError> {
    let manifest_bytes =
        serde_json::to_vec(manifest).map_err(|e| AppError::internal("backup manifest", &e))?;
    let err = |_: io::Error| io_err("backup_write");
    let mut db_file = File::open(db).map_err(err)?;
    let db_len = db_file.metadata().map_err(err)?.len();
    let mut out = BufWriter::new(File::create(tmp).map_err(err)?);
    out.write_all(&MAGIC).map_err(err)?;
    for part in [manifest_bytes.as_slice(), vault_json] {
        let len = u32::try_from(part.len()).map_err(|_| io_err("backup_write"))?;
        out.write_all(&len.to_le_bytes()).map_err(err)?;
        out.write_all(part).map_err(err)?;
    }
    out.write_all(&db_len.to_le_bytes()).map_err(err)?;
    let copied = io::copy(&mut db_file, &mut out).map_err(err)?;
    if copied != db_len {
        // База изменилась во время копирования: такая копия бесполезна.
        return Err(io_err("backup_write"));
    }
    out.flush().map_err(err)?;
    out.into_inner()
        .map_err(|e| err(e.into_error()))?
        .sync_all()
        .map_err(err)
}

fn read_section(input: &mut impl Read, max: u32) -> Result<Vec<u8>, AppError> {
    let mut len = [0; 4];
    input.read_exact(&mut len).map_err(|_| bad_file())?;
    let len = u32::from_le_bytes(len);
    if len > max {
        return Err(bad_file());
    }
    let mut buf = vec![0; usize::try_from(len).map_err(|_| bad_file())?];
    input.read_exact(&mut buf).map_err(|_| bad_file())?;
    Ok(buf)
}

fn header(input: &mut impl Read) -> Result<(Manifest, Vec<u8>), AppError> {
    let mut magic = [0; 8];
    input.read_exact(&mut magic).map_err(|_| bad_file())?;
    if magic != MAGIC {
        return Err(bad_file());
    }
    let manifest: Manifest =
        serde_json::from_slice(&read_section(input, MAX_MANIFEST)?).map_err(|_| bad_file())?;
    if manifest.format != FORMAT {
        return Err(AppError::Validation {
            message_key: "errors.backup.unsupported_format".into(),
            field: None,
        });
    }
    let vault_json = read_section(input, MAX_VAULT)?;
    Ok((manifest, vault_json))
}

/// Распаковывает копию в `out_dir` (папка уже создана): `vault.json` и `budget.db`.
/// Размер базы в заголовке должен совпасть с остатком файла: обрезанная копия отвергается.
///
/// # Errors
/// `Validation(errors.backup.bad_file)`, если файл не копия или обрезан; `Io` при записи.
pub fn extract(src: &Path, out_dir: &Path) -> Result<Manifest, AppError> {
    let mut input = BufReader::new(File::open(src).map_err(|_| io_err("backup_read"))?);
    let (manifest, vault_json) = header(&mut input)?;
    let mut len = [0; 8];
    input.read_exact(&mut len).map_err(|_| bad_file())?;
    let db_len = u64::from_le_bytes(len);
    fs::write(out_dir.join("vault.json"), &vault_json).map_err(|_| io_err("backup_write"))?;
    let mut db = BufWriter::new(
        File::create(out_dir.join("budget.db")).map_err(|_| io_err("backup_write"))?,
    );
    let copied =
        io::copy(&mut input.by_ref().take(db_len), &mut db).map_err(|_| io_err("backup_write"))?;
    let mut extra = [0; 1];
    let trailing = input.read(&mut extra).map_err(|_| bad_file())?;
    if copied != db_len || trailing != 0 {
        return Err(bad_file());
    }
    db.flush().map_err(|_| io_err("backup_write"))?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(dir: &Path) -> std::path::PathBuf {
        let db = dir.join("src.db");
        fs::write(&db, b"ciphertext-bytes").unwrap();
        let dest = dir.join("a.budgetbak");
        let manifest = Manifest::new("main".into(), "2026-11-13T10:00:00Z".into());
        write(&dest, &manifest, br#"{"version":1}"#, &db).unwrap();
        dest
    }

    #[test]
    fn round_trip_restores_both_files_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = sample(dir.path());
        assert!(!dir.path().join("a.budgetbak.tmp").exists());
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        assert_eq!(extract(&dest, &out).unwrap().vault_id, "main");
        assert_eq!(
            fs::read(out.join("budget.db")).unwrap(),
            b"ciphertext-bytes"
        );
        assert_eq!(
            fs::read(out.join("vault.json")).unwrap(),
            br#"{"version":1}"#
        );
    }

    #[test]
    fn truncated_padded_and_foreign_files_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let dest = sample(dir.path());
        let bytes = fs::read(&dest).unwrap();
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let cases: [(&str, Vec<u8>); 4] = [
            ("truncated", bytes[..bytes.len() - 3].to_vec()),
            ("padded", [bytes.clone(), vec![0]].concat()),
            ("foreign", b"PK\x03\x04 not ours at all".to_vec()),
            ("empty", Vec::new()),
        ];
        for (name, content) in cases {
            let path = dir.path().join(format!("{name}.budgetbak"));
            fs::write(&path, content).unwrap();
            assert!(extract(&path, &out).is_err(), "{name}");
        }
    }

    #[test]
    fn oversized_declared_sections_are_rejected_before_allocating() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.budgetbak");
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        fs::write(&path, bytes).unwrap();
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        assert!(extract(&path, &out).is_err());
    }
}

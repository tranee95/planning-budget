//! Перенос данных из папки прежнего `identifier`.
//!
//! Активен только в публичной сборке: `export-public.mjs` подставляет прежний идентификатор в
//! [`LEGACY_IDENTIFIER`]. В приватной сборке перенос выключен.
//!
//! Копируются только файлы сейфа и данных; старая папка остаётся нетронутой. Каждый файл
//! сначала пишется во временное имя, проверяется по размеру и переименовывается; `vault.json`
//! — последним, поэтому по нему видно, что перенос завершён. При сбое созданное удаляется.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Прежний идентификатор приложения; `None` — переносить нечего.
pub const LEGACY_IDENTIFIER: Option<&str> = Some("private-budget");

const TMP_SUFFIX: &str = ".migrating";
const VAULT: &str = "vault.json";
/// Файлы в корне папки данных; `vault.json` обрабатывается отдельно, последним.
const FILES: [&str; 3] = ["budget.db", "budget.db-wal", "ui-prefs.json"];
const BACKUPS_DIR: &str = "backups";

/// Итог попытки переноса.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Старой папки с сейфом нет или перенос не нужен.
    Skipped,
    /// Данные перенесены, число файлов.
    Migrated(usize),
}

/// Папка данных прежнего идентификатора рядом с текущей.
pub fn legacy_dir(data_dir: &Path, legacy_identifier: &str) -> Option<PathBuf> {
    Some(data_dir.parent()?.join(legacy_identifier))
}

/// Переносит данные из `old` в `new`, если в `old` есть сейф, а в `new` его ещё нет.
///
/// # Errors
/// Ошибка ввода-вывода; созданное при этом уже удалено, `old` не менялась.
pub fn migrate(old: &Path, new: &Path) -> io::Result<Outcome> {
    if !old.join(VAULT).is_file() || new.join(VAULT).exists() || old == new {
        return Ok(Outcome::Skipped);
    }
    fs::create_dir_all(new)?;
    let mut created: Vec<PathBuf> = Vec::new();
    match copy_all(old, new, &mut created) {
        Ok(count) => Ok(Outcome::Migrated(count)),
        Err(err) => {
            for path in &created {
                let _ = fs::remove_file(path);
            }
            Err(err)
        }
    }
}

fn copy_all(old: &Path, new: &Path, created: &mut Vec<PathBuf>) -> io::Result<usize> {
    let mut jobs: Vec<(PathBuf, PathBuf)> = Vec::new();
    for name in FILES {
        if old.join(name).is_file() {
            jobs.push((old.join(name), new.join(name)));
        }
    }
    let backups = old.join(BACKUPS_DIR);
    if backups.is_dir() {
        fs::create_dir_all(new.join(BACKUPS_DIR))?;
        for entry in fs::read_dir(&backups)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                jobs.push((entry.path(), new.join(BACKUPS_DIR).join(entry.file_name())));
            }
        }
    }
    jobs.push((old.join(VAULT), new.join(VAULT)));

    for (from, to) in &jobs {
        let tmp = tmp_name(to);
        created.push(tmp.clone());
        fs::copy(from, &tmp)?;
        if fs::metadata(from)?.len() != fs::metadata(&tmp)?.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "size mismatch"));
        }
        fs::rename(&tmp, to)?;
        created.push(to.clone());
    }
    Ok(jobs.len())
}

fn tmp_name(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(TMP_SUFFIX);
    PathBuf::from(name)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "тесты: паника и есть провал")]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn old_dir() -> (TempDir, PathBuf, PathBuf) {
        let root = TempDir::new().unwrap();
        let old = root.path().join("private-budget");
        let new = root.path().join("planning-budget");
        fs::create_dir_all(old.join("backups")).unwrap();
        fs::write(old.join("vault.json"), b"vault").unwrap();
        fs::write(old.join("budget.db"), b"db-bytes").unwrap();
        fs::write(old.join("ui-prefs.json"), b"{}").unwrap();
        fs::write(old.join("backups").join("a.budgetbak"), b"zip").unwrap();
        fs::create_dir_all(old.join("logs")).unwrap();
        fs::write(old.join("logs").join("app.log"), b"log").unwrap();
        (root, old, new)
    }

    #[test]
    fn copies_data_files_and_keeps_the_old_folder() {
        let (_root, old, new) = old_dir();
        assert_eq!(migrate(&old, &new).unwrap(), Outcome::Migrated(4));
        assert_eq!(fs::read(new.join("vault.json")).unwrap(), b"vault");
        assert_eq!(fs::read(new.join("budget.db")).unwrap(), b"db-bytes");
        assert_eq!(
            fs::read(new.join("backups").join("a.budgetbak")).unwrap(),
            b"zip"
        );
        assert!(!new.join("logs").exists(), "логи не переносятся");
        assert!(old.join("vault.json").is_file(), "старая папка осталась");
        let leftovers = fs::read_dir(&new)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(TMP_SUFFIX))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn second_run_does_nothing() {
        let (_root, old, new) = old_dir();
        migrate(&old, &new).unwrap();
        assert_eq!(migrate(&old, &new).unwrap(), Outcome::Skipped);
    }

    #[test]
    fn does_not_touch_a_new_folder_that_already_has_a_vault() {
        let (_root, old, new) = old_dir();
        fs::create_dir_all(&new).unwrap();
        fs::write(new.join("vault.json"), b"fresh").unwrap();
        assert_eq!(migrate(&old, &new).unwrap(), Outcome::Skipped);
        assert_eq!(fs::read(new.join("vault.json")).unwrap(), b"fresh");
        assert!(!new.join("budget.db").exists());
    }

    #[test]
    fn missing_old_folder_is_skipped() {
        let root = TempDir::new().unwrap();
        let outcome = migrate(&root.path().join("none"), &root.path().join("new")).unwrap();
        assert_eq!(outcome, Outcome::Skipped);
    }

    #[test]
    fn failure_rolls_back_created_files() {
        let (_root, old, new) = old_dir();
        // На месте временного файла vault.json лежит папка: сбой на последнем шаге.
        fs::create_dir_all(new.join("vault.json.migrating")).unwrap();
        assert!(migrate(&old, &new).is_err());
        assert!(!new.join("budget.db").exists(), "созданное удалено");
        assert!(!new.join("ui-prefs.json").exists());
        assert!(!new.join("vault.json").exists());
        assert!(old.join("vault.json").is_file());
    }

    #[test]
    fn legacy_dir_sits_next_to_the_current_one() {
        let dir = Path::new("root").join("planning-budget");
        assert_eq!(
            legacy_dir(&dir, "private-budget").unwrap(),
            Path::new("root").join("private-budget")
        );
    }
}

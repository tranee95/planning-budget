//! `budget-app --self-test`: CI-проверка на каждой ОС.
//! Создаёт временную зашифрованную базу, пишет и читает, проверяет шифрование, удаляет её.

use budget_storage::{Db, StorageError};

const KEY: [u8; 32] = [7; 32];
const OTHER_KEY: [u8; 32] = [9; 32];

/// Причина провала self-test: короткий код без данных.
#[derive(Debug, thiserror::Error)]
pub enum SelfTestError {
    #[error("io")]
    Io(#[from] std::io::Error),
    #[error("storage")]
    Storage(#[from] StorageError),
    #[error("check failed: {0}")]
    Check(&'static str),
}

/// Запускает проверки. Временный каталог удаляется в любом случае.
///
/// # Errors
/// Первая не прошедшая проверка.
pub fn run() -> Result<(), SelfTestError> {
    let dir = std::env::temp_dir().join(format!("budget-self-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let result = checks(&dir.join("budget.db"));
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn checks(path: &std::path::Path) -> Result<(), SelfTestError> {
    let db = Db::create(path, &KEY)?;
    db.conn()
        .execute_batch("CREATE TABLE t(v TEXT); INSERT INTO t VALUES ('ok');")
        .map_err(StorageError::from)?;
    let v: String = db
        .conn()
        .query_row("SELECT v FROM t", [], |r| r.get(0))
        .map_err(StorageError::from)?;
    if v != "ok" {
        return Err(SelfTestError::Check("read back"));
    }
    drop(db);
    if std::fs::read(path)?.starts_with(b"SQLite format 3\0") {
        return Err(SelfTestError::Check("file is not encrypted"));
    }
    if !matches!(Db::open(path, &OTHER_KEY), Err(StorageError::KeyRejected)) {
        return Err(SelfTestError::Check("wrong key accepted"));
    }
    Ok(())
}

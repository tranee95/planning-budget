//! Спайк: SQLCipher вшит в сборку, файл зашифрован, FTS5 доступен.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use planning_budget_storage::{Db, StorageError};
use rusqlite::Connection;

const KEY: [u8; 32] = [7; 32];
const OTHER_KEY: [u8; 32] = [9; 32];

fn create_db(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let path = dir.path().join("budget.db");
    let db = Db::create(&path, &KEY).unwrap();
    db.conn()
        .execute_batch("CREATE TABLE t(v TEXT); INSERT INTO t VALUES ('secret');")
        .unwrap();
    path
}

#[test]
fn file_is_not_plain_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_db(&dir);
    let head = std::fs::read(&path).unwrap();
    assert!(!head.starts_with(b"SQLite format 3\0"));
}

#[test]
fn opens_with_right_key_and_reads_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_db(&dir);
    let db = Db::open(&path, &KEY).unwrap();
    let v: String = db
        .conn()
        .query_row("SELECT v FROM t", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, "secret");
}

#[test]
fn wrong_key_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_db(&dir);
    assert!(matches!(
        Db::open(&path, &OTHER_KEY),
        Err(StorageError::KeyRejected)
    ));
}

#[test]
fn opening_without_key_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_db(&dir);
    let plain = Connection::open(&path).unwrap();
    let res = plain.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    });
    assert!(res.is_err());
}

#[test]
fn fts5_is_available() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    db.conn()
        .execute_batch(
            "CREATE VIRTUAL TABLE s USING fts5(title, tokenize = \"unicode61 remove_diacritics 2\", prefix = '2 3');
             INSERT INTO s VALUES ('Лента продукты');",
        )
        .unwrap();
    let n: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM s WHERE s MATCH 'лен*'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 1);
}

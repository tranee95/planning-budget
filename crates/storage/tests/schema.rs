//! Миграция 001, сид и триггеры FTS.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use chrono::{TimeZone, Utc};
use planning_budget_storage::Db;
use rusqlite::{Connection, params};
use tempfile::TempDir;

const KEY: [u8; 32] = [5; 32];
/// Число файлов миграций: версия схемы равна ему, константа не требует правки с новой миграцией.
fn latest_version() -> i64 {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let files = std::fs::read_dir(dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "sql")
        })
        .count();
    i64::try_from(files).unwrap()
}

const NOW: &str = "2026-10-01T12:00:00Z";

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

fn open() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let db = Db::create(&dir.path().join("budget.db"), &KEY).unwrap();
    (dir, db)
}

fn seeded() -> (TempDir, Db) {
    let (dir, mut db) = open();
    db.seed_defaults(now()).unwrap();
    (dir, db)
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

fn category_id(conn: &Connection, name: &str) -> i64 {
    conn.query_row("SELECT id FROM categories WHERE name = ?1", [name], |r| {
        r.get(0)
    })
    .unwrap()
}

fn add_tx(conn: &Connection, category: i64, title: &str, comment: Option<&str>) -> i64 {
    conn.execute(
        "INSERT INTO transactions (month, category_id, title, amount, status, comment, created_at, updated_at)
         VALUES ('2026-09', ?1, ?2, 10000, 'paid', ?3, ?4, ?4)",
        params![category, title, comment, NOW],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// ref_id трат, найденных по запросу в FTS (запрос — уже готовое FTS-выражение).
fn search(conn: &Connection, query: &str) -> Vec<i64> {
    let mut stmt = conn
        .prepare("SELECT ref_id FROM search_fts WHERE search_fts MATCH ?1 AND kind = 'tx' ORDER BY ref_id")
        .unwrap();
    stmt.query_map([query], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn migration_sets_user_version_and_passes_integrity_checks() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    assert_eq!(count(conn, "PRAGMA user_version"), latest_version());
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    assert_eq!(
        count(conn, "SELECT count(*) FROM pragma_foreign_key_check"),
        0
    );
}

#[test]
fn reopening_does_not_reapply_migrations_or_touch_data() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("budget.db");
    {
        let mut db = Db::create(&path, &KEY).unwrap();
        db.seed_defaults(now()).unwrap();
    }
    let db = Db::open(&path, &KEY).unwrap();
    assert_eq!(count(db.conn(), "PRAGMA user_version"), latest_version());
    assert_eq!(count(db.conn(), "SELECT count(*) FROM categories"), 17);
}

#[test]
fn pragmas_follow_the_documented_set() {
    let (_dir, db) = open();
    let conn = db.conn();
    assert_eq!(count(conn, "PRAGMA foreign_keys"), 1);
    assert_eq!(count(conn, "PRAGMA secure_delete"), 1);
    assert_eq!(count(conn, "PRAGMA temp_store"), 2, "MEMORY");
    assert_eq!(count(conn, "PRAGMA busy_timeout"), 2000);
    assert_eq!(count(conn, "PRAGMA cache_size"), -16_384);
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn seed_creates_categories_limits_and_settings() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    assert_eq!(count(conn, "SELECT count(*) FROM categories"), 17);
    assert_eq!(
        count(
            conn,
            "SELECT count(*) FROM categories WHERE kind = 'savings'"
        ),
        1
    );
    // Первый старт обобщённый: лимитов нет, их задаёт пользователь; у сбережений лимита нет и в
    // дальнейшем, вместо него процент плана.
    assert_eq!(count(conn, "SELECT count(*) FROM category_limits"), 0);
    assert_eq!(
        count(conn, "SELECT count(*) FROM savings_category_rates"),
        1
    );
    assert_eq!(count(conn, "SELECT count(*) FROM settings"), 9);
    let autolock: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'security.autolock_minutes'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(autolock, "5");
}

#[test]
fn setting_returns_stored_json_or_none() {
    let (_dir, db) = seeded();
    assert_eq!(
        db.setting("security.autolock_minutes").unwrap().as_deref(),
        Some("5")
    );
    assert_eq!(db.setting("no.such.key").unwrap(), None);
}

#[test]
fn rekey_moves_the_whole_database_to_the_new_key() {
    const NEW_KEY: [u8; 32] = [6; 32];
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("budget.db");
    {
        let mut db = Db::create(&path, &KEY).unwrap();
        db.seed_defaults(now()).unwrap();
        let cat = category_id(db.conn(), "Продукты");
        // Данные лежат и в основном файле, и в WAL (после checkpoint) — перешифровать надо всё.
        add_tx(db.conn(), cat, "Лента", Some("комментарий"));
        db.rekey(&NEW_KEY).unwrap();
        // Соединение продолжает работать и пишет уже под новым ключом.
        add_tx(db.conn(), cat, "Магнит", None);
    }
    assert!(matches!(
        Db::open(&path, &KEY),
        Err(planning_budget_storage::StorageError::KeyRejected)
    ));
    let db = Db::open(&path, &NEW_KEY).unwrap();
    let conn = db.conn();
    assert_eq!(count(conn, "SELECT count(*) FROM categories"), 17);
    assert_eq!(count(conn, "SELECT count(*) FROM transactions"), 2);
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    assert_eq!(search(conn, "\"лент\"*").len(), 1, "FTS survives rekey");
    assert_eq!(search(conn, "\"магн\"*").len(), 1);
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn rekeyed_database_file_is_still_encrypted() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("budget.db");
    {
        let mut db = Db::create(&path, &KEY).unwrap();
        db.rekey(&[8; 32]).unwrap();
    }
    let head = std::fs::read(&path).unwrap();
    assert!(!head.starts_with(b"SQLite format 3\0"));
}

#[test]
fn seed_colors_come_from_the_palette_in_sort_order() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    assert_eq!(
        count(conn, "SELECT count(DISTINCT color) FROM categories"),
        10
    );
    let first: String = conn
        .query_row(
            "SELECT color FROM categories WHERE sort_order = 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let eleventh: String = conn
        .query_row(
            "SELECT color FROM categories WHERE sort_order = 10",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(first, eleventh);
}

#[test]
fn seed_is_idempotent_and_keeps_user_changes() {
    let (_dir, mut db) = seeded();
    db.conn()
        .execute(
            "UPDATE settings SET value = '15' WHERE key = 'security.autolock_minutes'",
            [],
        )
        .unwrap();
    db.conn()
        .execute("DELETE FROM settings WHERE key = 'currency'", [])
        .unwrap();
    db.seed_defaults(now()).unwrap();
    let conn = db.conn();
    assert_eq!(count(conn, "SELECT count(*) FROM categories"), 17);
    assert_eq!(count(conn, "SELECT count(*) FROM category_limits"), 0);
    assert_eq!(count(conn, "SELECT count(*) FROM settings"), 9);
    let autolock: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'security.autolock_minutes'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(autolock, "15");
}

#[test]
fn seed_creates_the_standard_dashboard_once() {
    let (_dir, mut db) = seeded();
    db.seed_defaults(now()).unwrap();
    let conn = db.conn();
    assert_eq!(count(conn, "SELECT count(*) FROM dashboards"), 1);
    assert_eq!(
        count(conn, "SELECT count(*) FROM dashboards WHERE is_default = 1"),
        1
    );
    assert_eq!(count(conn, "SELECT count(*) FROM charts"), 9);

    let expected = planning_budget_core::analytics::standard_dashboard();
    let mut stmt = conn
        .prepare("SELECT spec, x, y, w, h FROM charts ORDER BY id")
        .unwrap();
    let rows: Vec<(String, u8, u8, u8, u8)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (row, placement) in rows.iter().zip(&expected) {
        let spec: planning_budget_core::analytics::ChartSpec =
            serde_json::from_str(&row.0).unwrap();
        assert_eq!(spec, placement.spec);
        assert_eq!(
            (row.1, row.2, row.3, row.4),
            (placement.x, placement.y, placement.w, placement.h)
        );
    }

    conn.execute("DELETE FROM dashboards", []).unwrap();
    assert_eq!(count(conn, "SELECT count(*) FROM charts"), 0, "каскад");
}

#[test]
fn table_constraints_reject_bad_rows() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let insert = |month: &str, date: Option<&str>, amount: i64, cat: i64| {
        conn.execute(
            "INSERT INTO transactions (month, date, category_id, title, amount, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'x', ?4, 'paid', ?5, ?5)",
            params![month, date, cat, amount, NOW],
        )
    };
    assert!(insert("2026-09", None, 100, cat).is_ok());
    assert!(insert("2026-09", Some("2026-09-15"), 100, cat).is_ok());
    assert!(
        insert("2026-09", Some("2026-08-15"), 100, cat).is_err(),
        "date vs month"
    );
    assert!(insert("2026-09", None, 0, cat).is_err(), "amount > 0");
    assert!(insert("2026-09", None, 100, 9999).is_err(), "foreign key");
    assert!(
        conn.execute(
            "INSERT INTO categories (name, kind, color, sort_order, created_at, updated_at)
             VALUES ('продукты', 'wants', '#000000', 99, ?1, ?1)",
            [NOW],
        )
        .is_err(),
        "category name is unique ignoring case"
    );
}

#[test]
fn fingerprint_is_unique_only_among_live_rows() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let id = add_tx(conn, cat, "a", None);
    conn.execute(
        "UPDATE transactions SET fingerprint = 'f1' WHERE id = ?1",
        [id],
    )
    .unwrap();
    let id2 = add_tx(conn, cat, "b", None);
    assert!(
        conn.execute(
            "UPDATE transactions SET fingerprint = 'f1' WHERE id = ?1",
            [id2]
        )
        .is_err()
    );
    conn.execute(
        "UPDATE transactions SET deleted_at = ?2 WHERE id = ?1",
        params![id, NOW],
    )
    .unwrap();
    conn.execute(
        "UPDATE transactions SET fingerprint = 'f1' WHERE id = ?1",
        [id2],
    )
    .unwrap();
}

#[test]
fn views_hide_soft_deleted_rows() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let id = add_tx(conn, cat, "a", None);
    add_tx(conn, cat, "b", None);
    conn.execute(
        "UPDATE transactions SET deleted_at = ?2 WHERE id = ?1",
        params![id, NOW],
    )
    .unwrap();
    assert_eq!(count(conn, "SELECT count(*) FROM v_transactions"), 1);
    assert_eq!(count(conn, "SELECT count(*) FROM transactions"), 2);
}

#[test]
fn norm_function_folds_yo_and_case() {
    let (_dir, db) = open();
    let text: String = db
        .conn()
        .query_row("SELECT norm('  Лёд   ПЕРЕВОД ')", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(text, "лед перевод");
}

#[test]
fn fts_finds_new_transaction_by_prefix_ignoring_case_and_yo() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let id = add_tx(conn, cat, "Лёгкий ужин", Some("Скидка по Карте"));
    assert_eq!(search(conn, "\"легк\"*"), [id]);
    assert_eq!(search(conn, "\"карт\"*"), [id], "comment is indexed");
    assert_eq!(
        search(conn, "\"продукт\"*"),
        [id],
        "category name is indexed"
    );
    assert!(search(conn, "\"молоко\"*").is_empty());
}

#[test]
fn fts_follows_updates_soft_delete_and_hard_delete() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let id = add_tx(conn, cat, "Лента", None);
    assert_eq!(search(conn, "\"лент\"*"), [id]);

    conn.execute(
        "UPDATE transactions SET title = 'Магнит' WHERE id = ?1",
        [id],
    )
    .unwrap();
    assert!(search(conn, "\"лент\"*").is_empty());
    assert_eq!(search(conn, "\"магн\"*"), [id]);

    conn.execute(
        "UPDATE transactions SET deleted_at = ?2 WHERE id = ?1",
        params![id, NOW],
    )
    .unwrap();
    assert!(search(conn, "\"магн\"*").is_empty());

    conn.execute(
        "UPDATE transactions SET deleted_at = NULL WHERE id = ?1",
        [id],
    )
    .unwrap();
    assert_eq!(search(conn, "\"магн\"*"), [id]);
    conn.execute("DELETE FROM transactions WHERE id = ?1", [id])
        .unwrap();
    assert!(search(conn, "\"магн\"*").is_empty());
}

#[test]
fn fts_reindexes_on_category_rename_and_on_tags() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let id = add_tx(conn, cat, "Ужин", None);

    conn.execute("UPDATE categories SET name = 'Еда' WHERE id = ?1", [cat])
        .unwrap();
    assert!(search(conn, "\"продукт\"*").is_empty());
    assert_eq!(search(conn, "\"еда\"*"), [id]);

    conn.execute("INSERT INTO tags (name) VALUES ('Командировка')", [])
        .unwrap();
    let tag = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
        [id, tag],
    )
    .unwrap();
    assert_eq!(search(conn, "\"командир\"*"), [id]);
    conn.execute(
        "DELETE FROM transaction_tags WHERE transaction_id = ?1",
        [id],
    )
    .unwrap();
    assert!(search(conn, "\"командир\"*").is_empty());
    assert_eq!(search(conn, "\"ужин\"*"), [id], "row itself stays indexed");
}

#[test]
fn fts_indexes_incomes_without_clashing_with_transaction_rowids() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let tx = add_tx(conn, cat, "Зарплата", None);
    conn.execute(
        "INSERT INTO incomes (month, source_name, amount, status, created_at, updated_at)
         VALUES ('2026-09', 'Зарплата 05', 100000, 'received', ?1, ?1)",
        [NOW],
    )
    .unwrap();
    assert_eq!(search(conn, "\"зарплат\"*"), [tx]);
    let incomes: i64 = conn
        .query_row(
            "SELECT count(*) FROM search_fts WHERE search_fts MATCH '\"зарплат\"*' AND kind = 'inc'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(incomes, 1);
}

#[test]
fn seed_and_search_dataset_stays_consistent_after_many_inserts() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    conn.execute_batch("BEGIN").unwrap();
    for n in 0..2000 {
        add_tx(conn, cat, &format!("Покупка {n}"), None);
    }
    conn.execute_batch("COMMIT").unwrap();
    assert_eq!(
        count(conn, "SELECT count(*) FROM search_fts WHERE kind = 'tx'"),
        2000
    );
    assert_eq!(search(conn, "\"покупка\" \"1999\"").len(), 1);
}

#[test]
fn open_refuses_a_missing_file_instead_of_creating_an_empty_database() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("budget.db");
    assert!(matches!(
        Db::open(&path, &KEY),
        Err(planning_budget_storage::StorageError::Missing)
    ));
    assert!(!path.exists(), "nothing was created");
}

#[test]
fn tag_names_are_unique_ignoring_cyrillic_case_and_yo() {
    let (_dir, db) = open();
    let conn = db.conn();
    conn.execute("INSERT INTO tags (name) VALUES ('Отпуск')", [])
        .unwrap();
    assert!(
        conn.execute("INSERT INTO tags (name) VALUES ('отпуск')", [])
            .is_err()
    );
    conn.execute("INSERT INTO tags (name) VALUES ('Ёлка')", [])
        .unwrap();
    assert!(
        conn.execute("INSERT INTO tags (name) VALUES ('елка')", [])
            .is_err()
    );
}

#[test]
fn fts_reindexes_transactions_when_a_tag_is_renamed() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    let cat = category_id(conn, "Продукты");
    let tagged = add_tx(conn, cat, "Ужин", None);
    add_tx(conn, cat, "Обед", None);
    conn.execute("INSERT INTO tags (name) VALUES ('еда')", [])
        .unwrap();
    let tag = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
        [tagged, tag],
    )
    .unwrap();
    assert_eq!(search(conn, "\"еда\"*"), [tagged]);

    conn.execute("UPDATE tags SET name = 'перекус' WHERE id = ?1", [tag])
        .unwrap();
    assert!(search(conn, "\"еда\"*").is_empty());
    assert_eq!(search(conn, "\"перекус\"*"), [tagged]);
    assert_eq!(
        search(conn, "\"ужин\"*"),
        [tagged],
        "row itself stays indexed"
    );
}

#[test]
fn seed_gives_savings_category_a_rate_equal_to_target_norm() {
    let (_dir, db) = seeded();
    let conn = db.conn();
    assert_eq!(
        count(conn, "SELECT count(*) FROM savings_category_rates"),
        1
    );
    let (name, valid_from, rate): (String, String, i64) = conn
        .query_row(
            "SELECT c.name, r.valid_from, r.rate_bp FROM savings_category_rates r
             JOIN categories c ON c.id = r.category_id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (name.as_str(), valid_from.as_str(), rate),
        ("Сбережения", "2026-10", 1400)
    );
}

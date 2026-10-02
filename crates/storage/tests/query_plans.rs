//! Планы запросов горячего пути: индексы по `EXPLAIN QUERY PLAN`, а не «на всякий
//! случай». Тест падает, если запрос вернулся к полному сканированию или временной сортировке.
//! Тексты запросов повторяют `dataset.rs`, `transactions.rs` и `search_list.rs`: меняя там
//! запрос или индекс, меняйте и здесь.

#![allow(
    clippy::unwrap_used,
    reason = "тестовый файл: паника и есть провал теста"
)]

use budget_storage::Db;
use chrono::{TimeZone, Utc};
use tempfile::TempDir;

fn db_with_rows() -> (TempDir, Db) {
    let dir = TempDir::new().unwrap();
    let mut db = Db::create(&dir.path().join("budget.db"), &[5; 32]).unwrap();
    db.seed_defaults(Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap())
        .unwrap();
    let conn = db.conn();
    conn.execute_batch("BEGIN").unwrap();
    for i in 0..1_000 {
        conn.execute(
            "INSERT INTO transactions
                 (month, date, category_id, title, amount, status, source, sort_key, fingerprint,
                  created_at, updated_at)
             VALUES (?1, ?2, (SELECT min(id) FROM categories), 't', 100 + ?3, 'paid', 'import',
                     ?3, ?4, 'x', 'x')",
            rusqlite::params![
                format!("2026-{:02}", 1 + i % 12),
                format!("2026-{:02}-10", 1 + i % 12),
                i,
                format!("fp{i}")
            ],
        )
        .unwrap();
    }
    for i in 0..100 {
        conn.execute(
            "INSERT INTO incomes (month, date, source_name, amount, status, source, created_at, updated_at)
             VALUES (?1, ?2, 'зарплата', 1000, 'received', 'import', 'x', 'x')",
            rusqlite::params![format!("2026-{:02}", 1 + i % 12), format!("2026-{:02}-10", 1 + i % 12)],
        )
        .unwrap();
    }
    conn.execute_batch("COMMIT; ANALYZE").unwrap();
    (dir, db)
}

fn plan(db: &Db, sql: &str) -> String {
    let mut stmt = db
        .conn()
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .unwrap();
    let rows: Vec<String> = stmt
        .query_map(
            rusqlite::params_from_iter(vec!["2026-01"; sql.matches('?').count()]),
            |r| r.get::<_, String>(3),
        )
        .unwrap()
        .map(Result::unwrap)
        .collect();
    rows.join(" | ")
}

fn assert_indexed(db: &Db, sql: &str, index: &str) {
    let plan = plan(db, sql);
    assert!(plan.contains(index), "ожидался индекс {index}: {plan}");
    assert!(
        !plan.contains("SCAN transactions ") || plan.contains("USING"),
        "{plan}"
    );
    assert!(
        !plan.contains("TEMP B-TREE"),
        "временная сортировка: {plan}"
    );
}

#[test]
fn ledger_read_uses_the_covering_index_without_sorting() {
    let (_dir, db) = db_with_rows();
    assert_indexed(
        &db,
        "SELECT id, month, category_id, title, amount, status FROM v_transactions
         WHERE month BETWEEN '2026-01' AND '2026-12' ORDER BY month, category_id, sort_key, id",
        "ix_tx_ledger",
    );
}

#[test]
fn month_list_and_next_sort_key_use_the_ledger_index() {
    let (_dir, db) = db_with_rows();
    assert_indexed(
        &db,
        "SELECT * FROM v_transactions WHERE month = '2026-03' ORDER BY category_id, sort_key",
        "ix_tx_ledger",
    );
    assert_indexed(
        &db,
        "SELECT max(sort_key) FROM v_transactions WHERE month = '2026-03' AND category_id = 1",
        "ix_tx_ledger",
    );
}

#[test]
fn expenses_table_reads_the_list_index_backwards_without_sorting() {
    let (_dir, db) = db_with_rows();
    assert_indexed(
        &db,
        "SELECT t.id FROM v_transactions t JOIN categories c ON c.id = t.category_id
         ORDER BY t.month DESC, t.date DESC, t.id DESC LIMIT 5000",
        "ix_tx_list",
    );
}

#[test]
fn fingerprint_lookup_and_status_filter_use_indexes() {
    let (_dir, db) = db_with_rows();
    assert_indexed(
        &db,
        "SELECT id FROM transactions WHERE fingerprint = 'x' AND deleted_at IS NULL",
        "ux_tx_fingerprint",
    );
    assert_indexed(
        &db,
        "SELECT id FROM v_transactions WHERE month = '2026-03' AND status = 'paid'",
        "ix_tx_status",
    );
}

#[test]
fn full_text_search_goes_through_the_fts_index_and_primary_keys() {
    let (_dir, db) = db_with_rows();
    for sql in [
        "SELECT t.id FROM search_fts JOIN v_transactions t ON t.id = (search_fts.rowid >> 1)
         JOIN categories c ON c.id = t.category_id
         WHERE search_fts MATCH ? AND (search_fts.rowid & 1) = 0
         ORDER BY bm25(search_fts), t.month DESC, t.date DESC, t.id DESC LIMIT ?",
        "SELECT i.id FROM search_fts JOIN v_incomes i ON i.id = (search_fts.rowid >> 1)
         WHERE search_fts MATCH ? AND (search_fts.rowid & 1) = 1
         ORDER BY bm25(search_fts), i.month DESC, i.date DESC, i.id DESC LIMIT ?",
    ] {
        let plan = plan(&db, sql);
        // Сортировка по релевантности неизбежна, но идёт только по найденному.
        assert!(plan.contains("VIRTUAL TABLE INDEX"), "{plan}");
        assert!(plan.contains("INTEGER PRIMARY KEY"), "{plan}");
        assert!(
            !plan.contains("SCAN transactions") && !plan.contains("SCAN incomes"),
            "{plan}"
        );
    }
}

#[test]
fn tag_filter_is_driven_by_the_tag_index() {
    let (_dir, db) = db_with_rows();
    let plan = plan(
        &db,
        "SELECT t.id FROM v_transactions t JOIN categories c ON c.id = t.category_id
         WHERE t.id IN (SELECT tt.transaction_id FROM transaction_tags tt WHERE tt.tag_id = ?)
         ORDER BY t.month DESC, t.date DESC, t.id DESC LIMIT 5000",
    );
    assert!(plan.contains("ix_tt_tag"), "{plan}");
}

#[test]
fn incomes_by_month_use_the_month_index() {
    let (_dir, db) = db_with_rows();
    for sql in [
        "SELECT count(*), coalesce(sum(i.amount), 0) FROM v_incomes i WHERE i.month BETWEEN ? AND ?",
        "SELECT i.id FROM v_incomes i WHERE i.month BETWEEN ? AND ?
         ORDER BY i.month DESC, i.date DESC, i.id DESC LIMIT 5000",
    ] {
        let plan = plan(&db, sql);
        assert!(plan.contains("ix_inc_month"), "{plan}");
    }
}

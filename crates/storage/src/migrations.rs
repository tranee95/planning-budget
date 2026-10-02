//! Миграции (`user_version`) и SQL-функция `norm()` для FTS.

use rusqlite::Connection;
use rusqlite::functions::FunctionFlags;
use rusqlite_migration::{M, Migrations};

/// Только вперёд: старые файлы не редактируются, схема меняется новой миграцией.
pub(crate) fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../migrations/001_init.sql")),
        M::up(include_str!("../migrations/002_savings_rates.sql")),
        M::up(include_str!("../migrations/003_tx_order_index.sql")),
        M::up(include_str!("../migrations/004_tx_list_index.sql")),
        M::up(include_str!("../migrations/005_tag_index.sql")),
    ])
}

/// Нормализация для поиска: lower, ё→е, пробелы схлопнуты.
pub(crate) fn normalize(text: &str) -> String {
    text.to_lowercase()
        .replace('ё', "е")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `norm(text)` используется во view и триггерах FTS, поэтому должна быть
/// зарегистрирована до миграций и на каждом соединении.
pub(crate) fn register_functions(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "norm",
        1,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_INNOCUOUS,
        |ctx| {
            let text: Option<String> = ctx.get(0)?;
            Ok(text.map(|t| normalize(&t)))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_to_an_empty_database() {
        let mut conn = Connection::open_in_memory().unwrap();
        register_functions(&conn).unwrap();
        migrations().to_latest(&mut conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn migration_002_carries_old_overrides_over_to_savings_categories() {
        let mut conn = Connection::open_in_memory().unwrap();
        register_functions(&conn).unwrap();
        migrations().to_version(&mut conn, 1).unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, name, kind, color, sort_order, created_at, updated_at)
                 VALUES (1, 'Сбережения', 'savings', '#3AA567', 0, 'x', 'x'),
                        (2, 'Продукты', 'mandatory', '#3AA567', 1, 'x', 'x');
             INSERT INTO savings_plan_overrides (month, rate_bp) VALUES ('2026-03', 2000);",
        )
        .unwrap();
        migrations().to_latest(&mut conn).unwrap();
        let rows: Vec<(String, i64, i64)> = conn
            .prepare("SELECT month, category_id, rate_bp FROM savings_plan_overrides")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows, vec![("2026-03".to_owned(), 1, 2000)]);
    }

    #[test]
    fn normalize_lowercases_folds_yo_and_collapses_spaces() {
        assert_eq!(normalize("  Лёд   ЛЕНТА\t248 "), "лед лента 248");
    }
}

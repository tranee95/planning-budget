//! Миграции (`user_version`) и SQL-функция `norm()` для FTS.

use rusqlite::Connection;
use rusqlite::functions::FunctionFlags;
use rusqlite_migration::{M, Migrations};

/// Скрипты миграций по порядку. Новая миграция — файл `migrations/NNN_*.sql` и одна строка здесь;
/// тест сверяет число строк с числом файлов, поэтому забыть одно из двух нельзя.
const SCRIPTS: &[&str] = &[
    include_str!("../migrations/001_init.sql"),
    include_str!("../migrations/002_savings_rates.sql"),
    include_str!("../migrations/003_tx_order_index.sql"),
    include_str!("../migrations/004_tx_list_index.sql"),
    include_str!("../migrations/005_tag_index.sql"),
    include_str!("../migrations/006_month_plan.sql"),
    include_str!("../migrations/007_debts.sql"),
    include_str!("../migrations/008_savings_params.sql"),
    include_str!("../migrations/009_drop_bonds_settings.sql"),
];

/// Только вперёд: старые файлы не редактируются, схема меняется новой миграцией.
pub(crate) fn migrations() -> Migrations<'static> {
    Migrations::new(SCRIPTS.iter().map(|sql| M::up(sql)).collect())
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
        assert_eq!(usize::try_from(version).unwrap(), SCRIPTS.len());
    }

    #[test]
    fn every_migration_file_is_registered() {
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
        assert_eq!(files, SCRIPTS.len());
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

    /// Данные версии 5 с тратами, процентом плана и настройками облигаций.
    fn v5_database() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        register_functions(&conn).unwrap();
        migrations().to_version(&mut conn, 5).unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, name, kind, color, sort_order, created_at, updated_at, archived_at)
                 VALUES (1, 'Продукты', 'mandatory', '#3AA567', 0, 'x', 'x', NULL),
                        (2, 'Старое накопление', 'savings', '#3AA567', 1, 'x', 'x', '2026-01-01'),
                        (3, 'Сбережения', 'savings', '#3AA567', 2, 'x', 'x', NULL),
                        (4, 'Отпуск', 'savings', '#3AA567', 3, 'x', 'x', NULL);
             INSERT INTO transactions (month, category_id, title, amount, status, sort_key, created_at, updated_at)
                 VALUES ('2026-09', 1, 'Хлеб', 25000, 'paid', 0, 'x', 'x'),
                        ('2026-09', 3, 'Взнос', 1400000, 'paid', 0, 'x', 'x');
             INSERT INTO savings_category_rates (category_id, valid_from, rate_bp)
                 VALUES (3, '2026-01', 1400), (4, '2026-01', 500);
             INSERT INTO settings (key, value) VALUES
                 ('bonds.rate_bp', '1600'), ('bonds.coupon_tax_bp', '1300'),
                 ('bonds.initial_balance', '50000'), ('bonds.initial_month', '\"2026-01\"');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migration_008_moves_bonds_settings_to_the_first_active_savings_category() {
        let mut conn = v5_database();
        migrations().to_version(&mut conn, 8).unwrap();
        let params: Vec<(i64, i64, i64, i64, String)> = conn
            .prepare(
                "SELECT category_id, annual_rate_bp, tax_bp, initial_balance, initial_month
                 FROM savings_params ORDER BY category_id",
            )
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        // Первая неархивная по порядку — «Сбережения» (id 3); архивная и «Отпуск» без процентов.
        assert_eq!(
            params,
            vec![
                (2, 0, 0, 0, "2026-01".to_owned()),
                (3, 1600, 1300, 50_000, "2026-01".to_owned()),
                (4, 0, 0, 0, "2026-01".to_owned()),
            ]
        );
        // До миграции 009 ключи облигаций остаются в настройках.
        let kept: i64 = conn
            .query_row(
                "SELECT count(*) FROM settings WHERE key LIKE 'bonds.%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, 4);
    }

    #[test]
    fn migration_009_removes_bonds_settings_but_keeps_the_rest_and_the_params() {
        let mut conn = v5_database();
        migrations().to_latest(&mut conn).unwrap();
        let bonds: i64 = conn
            .query_row(
                "SELECT count(*) FROM settings WHERE key LIKE 'bonds.%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bonds, 0);
        let params: i64 = conn
            .query_row(
                "SELECT annual_rate_bp FROM savings_params WHERE category_id = 3",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(params, 1600, "ставка осталась у накопления");
    }

    #[test]
    fn migration_008_keeps_percent_history_and_adds_plan_kind() {
        let mut conn = v5_database();
        migrations().to_latest(&mut conn).unwrap();
        let rows: Vec<(i64, String, String, i64, Option<i64>)> = conn
            .prepare(
                "SELECT category_id, valid_from, plan_kind, rate_bp, amount
                 FROM savings_category_rates ORDER BY category_id",
            )
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (3, "2026-01".to_owned(), "percent".to_owned(), 1400, None),
                (4, "2026-01".to_owned(), "percent".to_owned(), 500, None),
            ]
        );
    }

    #[test]
    fn fixed_plan_kind_requires_an_amount_and_percent_forbids_it() {
        let mut conn = v5_database();
        migrations().to_latest(&mut conn).unwrap();
        let insert = |kind: &str, amount: &str| {
            conn.execute(
                &format!(
                    "INSERT INTO savings_category_rates (category_id, valid_from, plan_kind, rate_bp, amount)
                     VALUES (4, '2026-06', '{kind}', 0, {amount})"
                ),
                [],
            )
        };
        assert!(insert("fixed", "NULL").is_err());
        assert!(insert("percent", "5000").is_err());
        assert!(insert("fixed", "500000").is_ok());
    }

    #[test]
    fn migration_006_keeps_transactions_and_leaves_them_out_of_any_plan() {
        let mut conn = v5_database();
        migrations().to_latest(&mut conn).unwrap();
        let (count, planned, total): (i64, i64, i64) = conn
            .query_row(
                "SELECT count(*), count(planned_amount), sum(amount) FROM v_transactions",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((count, planned, total), (2, 0, 1_425_000));
        let locked: i64 = conn
            .query_row("SELECT count(*) FROM month_plans", [], |r| r.get(0))
            .unwrap();
        assert_eq!(locked, 0);
    }

    #[test]
    fn debt_schedule_rows_are_unique_per_month_and_cascade_with_the_debt() {
        let mut conn = Connection::open_in_memory().unwrap();
        register_functions(&conn).unwrap();
        migrations().to_latest(&mut conn).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             INSERT INTO debts (id, lender, amount, taken_month, created_at, updated_at)
                 VALUES (1, 'Карта', 3000, '2026-09', 'x', 'x');
             INSERT INTO debt_payments (debt_id, month, amount, status) VALUES (1, '2026-10', 1500, 'planned');",
        )
        .unwrap();
        let duplicate = conn.execute(
            "INSERT INTO debt_payments (debt_id, month, amount, status) VALUES (1, '2026-10', 1500, 'planned')",
            [],
        );
        assert!(duplicate.is_err());
        conn.execute("DELETE FROM debts WHERE id = 1", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT count(*) FROM debt_payments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn normalize_lowercases_folds_yo_and_collapses_spaces() {
        assert_eq!(normalize("  Лёд   ЛЕНТА\t248 "), "лед лента 248");
    }
}

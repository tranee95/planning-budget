//! Примитивы записи строк, общие для начального сида, dev-сида и переноса из xlsx.
//!
//! Знание о колонках таблиц живёт здесь: новая колонка — правка одной функции, а не трёх
//! параллельных `INSERT`. Функции принимают `&Connection` (в том числе через `Transaction`)
//! и готовят запросы через кэш, поэтому цикл вставок не перепарсивает SQL.

use rusqlite::{Connection, params};

use crate::StorageError;

pub(crate) struct NewCategory<'a> {
    /// `None` — идентификатор назначает SQLite.
    pub id: Option<i64>,
    pub name: &'a str,
    pub kind: &'a str,
    pub color: &'a str,
    pub sort_order: i64,
    pub note: Option<&'a str>,
    pub archived_at: Option<&'a str>,
    pub stamp: &'a str,
}

/// Возвращает идентификатор созданной категории.
pub(crate) fn insert_category(conn: &Connection, c: &NewCategory<'_>) -> Result<i64, StorageError> {
    conn.prepare_cached(
        "INSERT INTO categories (id, name, kind, color, sort_order, note, archived_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
    )?
    .execute(params![
        c.id,
        c.name,
        c.kind,
        c.color,
        c.sort_order,
        c.note,
        c.archived_at,
        c.stamp
    ])?;
    Ok(conn.last_insert_rowid())
}

pub(crate) fn insert_limit(
    conn: &Connection,
    category_id: i64,
    valid_from: &str,
    amount: i64,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO category_limits (category_id, valid_from, amount) VALUES (?1, ?2, ?3)",
    )?
    .execute(params![category_id, valid_from, amount])?;
    Ok(())
}

/// Лимит на месяц, который мог уже существовать (перенос из xlsx поверх сида).
pub(crate) fn upsert_limit(
    conn: &Connection,
    category_id: i64,
    valid_from: &str,
    amount: i64,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO category_limits (category_id, valid_from, amount) VALUES (?1, ?2, ?3)
         ON CONFLICT (category_id, valid_from) DO UPDATE SET amount = excluded.amount",
    )?
    .execute(params![category_id, valid_from, amount])?;
    Ok(())
}

/// Строка истории плана накопления: процент (`fixed_amount = None`) либо фиксированная сумма.
pub(crate) fn insert_rate(
    conn: &Connection,
    category_id: i64,
    valid_from: &str,
    rate_bp: i64,
    fixed_amount: Option<i64>,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_category_rates (category_id, valid_from, plan_kind, rate_bp, amount)
         VALUES (?1, ?2, CASE WHEN ?4 IS NULL THEN 'percent' ELSE 'fixed' END, ?3, ?4)",
    )?
    .execute(params![category_id, valid_from, rate_bp, fixed_amount])?;
    Ok(())
}

/// Параметры накопления: добавляет или заменяет строку категории.
pub(crate) fn upsert_savings_params(
    conn: &Connection,
    category_id: i64,
    annual_rate_bp: i64,
    tax_bp: i64,
    initial_balance: i64,
    initial_month: &str,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_params (category_id, annual_rate_bp, tax_bp, initial_balance, initial_month)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (category_id) DO UPDATE SET annual_rate_bp = excluded.annual_rate_bp,
             tax_bp = excluded.tax_bp, initial_balance = excluded.initial_balance,
             initial_month = excluded.initial_month",
    )?
    .execute(params![
        category_id,
        annual_rate_bp,
        tax_bp,
        initial_balance,
        initial_month
    ])?;
    Ok(())
}

/// Простое накопление без процентов у каждой категории сбережений, у которой параметров ещё нет.
pub(crate) fn ensure_savings_params(
    conn: &Connection,
    initial_month: &str,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_params (category_id, annual_rate_bp, tax_bp, initial_balance, initial_month)
         SELECT id, 0, 0, 0, ?1 FROM categories
         WHERE kind = 'savings' AND id NOT IN (SELECT category_id FROM savings_params)",
    )?
    .execute([initial_month])?;
    Ok(())
}

/// План накопления процентом с месяца `valid_from`; сумма сбрасывается.
pub(crate) fn upsert_rate_percent(
    conn: &Connection,
    category_id: i64,
    valid_from: &str,
    rate_bp: i64,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_category_rates (category_id, valid_from, plan_kind, rate_bp, amount)
         VALUES (?1, ?2, 'percent', ?3, NULL)
         ON CONFLICT (category_id, valid_from) DO UPDATE SET plan_kind = 'percent',
             rate_bp = excluded.rate_bp, amount = NULL",
    )?
    .execute(params![category_id, valid_from, rate_bp])?;
    Ok(())
}

/// План накопления фиксированной суммой с месяца `valid_from`; процент сбрасывается.
pub(crate) fn upsert_rate_fixed(
    conn: &Connection,
    category_id: i64,
    valid_from: &str,
    amount: i64,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_category_rates (category_id, valid_from, plan_kind, rate_bp, amount)
         VALUES (?1, ?2, 'fixed', 0, ?3)
         ON CONFLICT (category_id, valid_from) DO UPDATE SET plan_kind = 'fixed',
             rate_bp = 0, amount = excluded.amount",
    )?
    .execute(params![category_id, valid_from, amount])?;
    Ok(())
}

/// Процент плана, только если до `valid_from` у категории ещё нет ни одного.
pub(crate) fn insert_rate_if_none(
    conn: &Connection,
    category_id: i64,
    valid_from: &str,
    rate_bp: i64,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_category_rates (category_id, valid_from, rate_bp)
         SELECT ?1, ?2, ?3 WHERE NOT EXISTS
             (SELECT 1 FROM savings_category_rates WHERE category_id = ?1 AND valid_from <= ?2)",
    )?
    .execute(params![category_id, valid_from, rate_bp])?;
    Ok(())
}

pub(crate) fn insert_override(
    conn: &Connection,
    month: &str,
    category_id: i64,
    rate_bp: i64,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO savings_plan_overrides (month, category_id, rate_bp) VALUES (?1, ?2, ?3)",
    )?
    .execute(params![month, category_id, rate_bp])?;
    Ok(())
}

/// Значение настройки — JSON-текст.
pub(crate) fn upsert_setting(
    conn: &Connection,
    key: &str,
    value: &str,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )?
    .execute(params![key, value])?;
    Ok(())
}

pub(crate) struct NewTransaction<'a> {
    pub id: Option<i64>,
    pub month: &'a str,
    pub category_id: i64,
    pub title: &'a str,
    pub amount: i64,
    pub status: &'a str,
    /// Источник записи: `seed`, `legacy`.
    pub source: &'a str,
    pub stamp: &'a str,
}

/// Новая трата встаёт в конец блока своей категории (`sort_key`).
pub(crate) fn insert_transaction(
    conn: &Connection,
    t: &NewTransaction<'_>,
) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO transactions
             (id, month, category_id, title, amount, status, source, sort_key, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                 (SELECT COALESCE(max(sort_key) + 1, 0) FROM transactions
                  WHERE month = ?2 AND category_id = ?3),
                 ?8, ?8)",
    )?
    .execute(params![
        t.id,
        t.month,
        t.category_id,
        t.title,
        t.amount,
        t.status,
        t.source,
        t.stamp
    ])?;
    Ok(())
}

pub(crate) struct NewIncome<'a> {
    pub id: Option<i64>,
    pub month: &'a str,
    pub source_name: &'a str,
    pub amount: i64,
    pub status: &'a str,
    pub source: &'a str,
    pub stamp: &'a str,
}

pub(crate) fn insert_income(conn: &Connection, i: &NewIncome<'_>) -> Result<(), StorageError> {
    conn.prepare_cached(
        "INSERT INTO incomes (id, month, source_name, amount, status, source, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
    )?
    .execute(params![
        i.id,
        i.month,
        i.source_name,
        i.amount,
        i.status,
        i.source,
        i.stamp
    ])?;
    Ok(())
}

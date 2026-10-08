//! Хранилище на SQLCipher: миграции, репозитории, поиск.

mod categories;
mod dashboards;
mod dataset;
mod debts;
mod devseed;
mod incomes;
mod legacy;
mod migrations;
mod month_plans;
mod plan;
mod platform;
mod records;
mod saved_filters;
mod search;
mod search_list;
mod seed;
mod settings;
mod suggest;
mod tags;
mod transactions;
mod writes;

pub use categories::{CategoryPatch, NewCategory};
pub use dashboards::{CardPlacement, ChartCard, Dashboard, GRID_COLUMNS};
pub use debts::{DebtPatch, DebtPaymentRecord, DebtRecord, NewDebt};
pub use incomes::{IncomePatch, IncomeRecord, IncomeUpdate, NewIncome};
pub use legacy::LegacyReport;
pub use month_plans::{PlanWizardInput, SavingsPlanInput};
pub use records::RecordSource;
pub use saved_filters::{FilterScreen, SavedFilter};
pub use search::{CategoryHit, Group, IncomeHit, MonthHit, SearchResult, TransactionHit};
pub use search_list::{IncomeList, LIST_LIMIT, TransactionList};
pub use suggest::TitleSuggestion;
pub use transactions::{NewTransaction, TransactionPatch, TransactionRecord, TransactionUpdate};

use std::path::Path;

use data_encoding::HEXLOWER;
use rusqlite::{Connection, OpenFlags};
use zeroize::Zeroizing;

/// Ошибки хранилища. Без путей и данных бюджета: текст ошибки может попасть в лог.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite error")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration failed")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("database key rejected")]
    KeyRejected,
    #[error("database file is missing")]
    Missing,
    #[error("cannot raise memory lock quota")]
    MemoryLock,
    #[error("embedded seed data is invalid")]
    SeedAsset,
    #[error("record not found")]
    NotFound,
    /// Ключ сообщения для UI, без данных бюджета.
    #[error("invalid input: {0}")]
    Invalid(&'static str),
    #[error("conflict: {0}")]
    Conflict(&'static str),
    #[error("stored value is malformed")]
    Corrupt,
}

impl StorageError {
    /// Нарушение ограничения SQLite (уникальность, внешний ключ, CHECK).
    pub(crate) fn is_constraint(&self) -> bool {
        matches!(
            self,
            Self::Sqlite(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation
        )
    }
}

/// Метка времени для `created_at` / `updated_at`: RFC 3339, UTC.
pub(crate) fn stamp(now: chrono::DateTime<chrono::Utc>) -> String {
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Открытая зашифрованная база.
#[derive(Debug)]
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Создаёт новую базу (или открывает существующую), выставляет PRAGMA строго в порядке
    /// и применяет миграции. Вызывается только при создании хранилища.
    ///
    /// # Errors
    /// [`StorageError::KeyRejected`], если файл уже есть и ключ к нему не подходит.
    pub fn create(path: &Path, key: &[u8; 32]) -> Result<Self, StorageError> {
        Self::open_with(path, key, OpenFlags::SQLITE_OPEN_CREATE)
    }

    /// Открывает существующую базу. Отсутствующий файл — ошибка, а не повод создать пустую:
    /// иначе вход при пропавшей `budget.db` молча показал бы пользователю пустой бюджет.
    ///
    /// # Errors
    /// [`StorageError::Missing`], если файла нет; [`StorageError::KeyRejected`], если ключ
    /// не подходит.
    pub fn open(path: &Path, key: &[u8; 32]) -> Result<Self, StorageError> {
        if !path.exists() {
            return Err(StorageError::Missing);
        }
        Self::open_with(path, key, OpenFlags::empty())
    }

    fn open_with(path: &Path, key: &[u8; 32], extra: OpenFlags) -> Result<Self, StorageError> {
        platform::prepare_memory_lock()?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | extra;
        let mut conn = Connection::open_with_flags(path, flags)?;
        {
            // PRAGMA key не принимает параметры; значение — только hex от DEK.
            let hex = Zeroizing::new(HEXLOWER.encode(key));
            let value = Zeroizing::new(format!("x'{}'", hex.as_str()));
            conn.pragma_update(None, "key", value.as_str())?;
        }
        conn.pragma_update(None, "cipher_memory_security", "ON")?;
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| StorageError::KeyRejected)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "secure_delete", "ON")?;
        // 16 МиБ: случайные чтения при поиске иначе упираются в расшифровку страниц.
        conn.pragma_update(None, "cache_size", -16_384)?;
        conn.pragma_update(None, "busy_timeout", 2000)?;
        migrations::register_functions(&conn)?;
        migrations::migrations().to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    /// Перешифровывает всю БД новым ключом.
    ///
    /// WAL сначала сбрасывается в основной файл и отключается: `PRAGMA rekey` в режиме WAL
    /// оставил бы часть страниц под старым ключом. После перешифровки WAL включается снова.
    ///
    /// # Errors
    /// Ошибка означает, что файл остался под старым ключом: всё, что может упасть,
    /// выполняется до `PRAGMA rekey`, а сама перешифровка идёт одной транзакцией.
    pub fn rekey(&mut self, new_key: &[u8; 32]) -> Result<(), StorageError> {
        // Во второй раз за сессию SQLCipher отвечает на TRUNCATE «table is locked»; соединение
        // единственное, поэтому обычный сброс переносит в файл все кадры WAL.
        if self
            .conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .is_err()
        {
            // Первый столбец — «занято»: ненулевое значение значит, что часть кадров осталась в WAL.
            let busy: i64 = self
                .conn
                .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |r| r.get(0))?;
            if busy != 0 {
                return Err(StorageError::Invalid("rekey.wal_busy"));
            }
        }
        let mode: String = self
            .conn
            .query_row("PRAGMA journal_mode = DELETE", [], |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("delete") {
            // Остаться в WAL нельзя: перешифровка оставила бы часть страниц под старым ключом.
            return Err(StorageError::Invalid("rekey.journal_mode"));
        }
        {
            let hex = Zeroizing::new(HEXLOWER.encode(new_key));
            let value = Zeroizing::new(format!("x'{}'", hex.as_str()));
            self.conn.pragma_update(None, "rekey", value.as_str())?;
        }
        // С этого места файл уже под новым ключом, и вызывающий обязан сохранить новые слоты.
        // Возврат в WAL — удобство, а не условие успеха: следующее открытие включит его само.
        let _ = self.conn.pragma_update(None, "journal_mode", "WAL");
        Ok(())
    }

    /// JSON-значение настройки или `None`, если ключа нет.
    ///
    /// # Errors
    /// Ошибка SQLite.
    pub fn setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    /// Соединение для репозиториев и тестов.
    #[must_use]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        // Ошибку оптимизатора при закрытии сессии обрабатывать нечем и незачем.
        let _ = self.conn.execute_batch("PRAGMA optimize");
    }
}

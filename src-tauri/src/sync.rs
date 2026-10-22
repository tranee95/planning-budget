//! Блокировки, переживающие панику.
//!
//! Паника в рабочем потоке отравляет мьютекс; данные под ним остаются целыми (транзакция
//! откатывается при drop), поэтому отравление игнорируется, а не превращается в новую панику.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(crate) trait MutexExt<T> {
    /// `lock()` без паники на отравленном мьютексе.
    fn lock_unpoisoned(&self) -> MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for Mutex<T> {
    fn lock_unpoisoned(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(crate) trait RwLockExt<T> {
    /// `read()` без паники на отравленной блокировке.
    fn read_unpoisoned(&self) -> RwLockReadGuard<'_, T>;
    /// `write()` без паники на отравленной блокировке.
    fn write_unpoisoned(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn read_unpoisoned(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write_unpoisoned(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(PoisonError::into_inner)
    }
}

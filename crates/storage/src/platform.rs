//! Платформенная настройка памяти под `cipher_memory_security`.

use crate::StorageError;

/// Готовит процесс к закреплению памяти SQLCipher. Выполняется один раз за запуск.
///
/// # Errors
/// [`StorageError::MemoryLock`], если Windows отказала в увеличении квоты.
pub(crate) fn prepare_memory_lock() -> Result<(), StorageError> {
    use std::sync::OnceLock;
    static RESULT: OnceLock<bool> = OnceLock::new();
    if *RESULT.get_or_init(raise_quota) {
        Ok(())
    } else {
        Err(StorageError::MemoryLock)
    }
}

#[cfg(windows)]
#[allow(unsafe_code, reason = "единственный вызов Win32, аргументы — числа")]
fn raise_quota() -> bool {
    use windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    const MIN_BYTES: usize = 64 << 20;
    const MAX_BYTES: usize = 256 << 20;
    // SAFETY: GetCurrentProcess возвращает псевдо-дескриптор текущего процесса,
    // закрывать его не нужно; остальные аргументы — обычные числа.
    unsafe { SetProcessWorkingSetSizeEx(GetCurrentProcess(), MIN_BYTES, MAX_BYTES, 0) != 0 }
}

#[cfg(not(windows))]
fn raise_quota() -> bool {
    true
}

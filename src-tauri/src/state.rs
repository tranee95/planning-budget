//! Состояние приложения: пути и сессия с открытой БД.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::Instant;

use planning_budget_storage::Db;
use planning_budget_vault::VaultStore;
use tauri::{AppHandle, Manager as _};

use crate::AppError;
use crate::idle::IdleTimer;

/// Выполняет блокирующую работу (Argon2, файлы, диалоги) вне async-рантайма.
///
/// Нужен командам, которые работают до разблокировки и потому не ходят через
/// `with_session`: `State` нельзя унести в `spawn_blocking`, а `AppHandle` можно.
///
/// # Errors
/// Ошибка из `f` или `Internal`, если рабочий поток завершился аварийно.
pub async fn blocking<T, F>(app: &AppHandle, f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> Result<T, AppError> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| AppError::internal("blocking worker", &e))?
}

/// Файлы в `app_data_dir()`.
#[derive(Debug, Clone)]
pub struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    #[must_use]
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    #[must_use]
    pub fn db(&self) -> PathBuf {
        self.data_dir.join("budget.db")
    }

    /// `budget.db`, WAL и shared-memory файл: удаляются вместе (`vault_reset`).
    #[must_use]
    pub fn db_files(&self) -> [PathBuf; 3] {
        ["budget.db", "budget.db-wal", "budget.db-shm"].map(|name| self.data_dir.join(name))
    }

    /// Настройки оболочки, нужные до разблокировки.
    #[must_use]
    pub fn prefs(&self) -> PathBuf {
        self.data_dir.join("ui-prefs.json")
    }
}

/// Разблокированная сессия. Закрытие соединения и зануление ключа — при drop `Db`.
#[derive(Debug)]
struct Session {
    db: Mutex<Db>,
    #[allow(dead_code, reason = "нужно idle-таймеру и логу длительности сессии")]
    opened_at: Instant,
}

impl Session {
    fn run<T>(&self, f: impl FnOnce(&mut Db) -> Result<T, AppError>) -> Result<T, AppError> {
        let mut db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut db)
    }
}

/// Общее состояние, лежит в `tauri::State`.
#[derive(Debug)]
pub struct AppState {
    paths: AppPaths,
    session: RwLock<Option<Arc<Session>>>,
    /// Мьютекс сериализует все операции над `vault.json` в этом процессе.
    vault: Mutex<VaultStore>,
    idle: IdleTimer,
    /// `security.lock_on_minimize`: копия настройки, чтобы обработчик окна не ходил в БД.
    lock_on_minimize: AtomicBool,
    /// Сохранение recovery-кода в файл разрешено только сразу после его выдачи.
    recovery_save_allowed: AtomicBool,
}

impl AppState {
    #[must_use]
    pub fn new(paths: AppPaths) -> Self {
        Self {
            vault: Mutex::new(VaultStore::new(paths.data_dir().to_path_buf())),
            paths,
            session: RwLock::new(None),
            idle: IdleTimer::new(),
            lock_on_minimize: AtomicBool::new(false),
            recovery_save_allowed: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    /// Сейф. Guard держится всю операцию «прочитать → изменить → записать».
    pub fn vault(&self) -> MutexGuard<'_, VaultStore> {
        self.vault.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[must_use]
    pub fn idle(&self) -> &IdleTimer {
        &self.idle
    }

    #[must_use]
    pub fn lock_on_minimize(&self) -> bool {
        self.lock_on_minimize.load(Ordering::Relaxed)
    }

    pub fn set_lock_on_minimize(&self, on: bool) {
        self.lock_on_minimize.store(on, Ordering::Relaxed);
    }

    pub fn allow_recovery_save(&self, allowed: bool) {
        self.recovery_save_allowed.store(allowed, Ordering::SeqCst);
    }

    #[must_use]
    pub fn recovery_save_allowed(&self) -> bool {
        self.recovery_save_allowed.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.current().is_some()
    }

    /// Начинает сессию. Предыдущая (если была) закрывается.
    pub fn open_session(&self, db: Db) {
        let session = Arc::new(Session {
            db: Mutex::new(db),
            opened_at: Instant::now(),
        });
        // Таймер сбрасывается до публикации сессии: иначе сторож успел бы увидеть
        // новую сессию со старой отметкой активности и сразу её закрыть.
        self.idle.touch();
        *self.session.write().unwrap_or_else(PoisonError::into_inner) = Some(session);
    }

    /// Закрывает сессию. `true`, если она была открыта.
    ///
    /// Запрос, уже выполняющийся в `with_session`, доработает: БД закроется, когда он
    /// отпустит соединение. Новые запросы получают `Locked` сразу.
    pub fn lock(&self) -> bool {
        self.allow_recovery_save(false);
        self.session
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .is_some()
    }

    fn current(&self) -> Option<Arc<Session>> {
        self.session
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Единственный путь к `Db` для команд с данными.
    ///
    /// Без открытой сессии возвращает [`AppError::Locked`], не вызывая `f`.
    /// Замыкание синхронное и выполняется в `spawn_blocking`: тяжёлые запросы
    /// не занимают async-рантайм, а `Mutex<Db>` не держится через `.await`.
    ///
    /// # Errors
    /// `Locked`, ошибка из `f` или `Internal`, если рабочий поток завершился аварийно.
    pub async fn with_session<T, F>(&self, f: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Db) -> Result<T, AppError> + Send + 'static,
    {
        let session = self.current().ok_or(AppError::Locked)?;
        tauri::async_runtime::spawn_blocking(move || session.run(f))
            .await
            .map_err(|e| AppError::internal("session worker", &e))?
    }

    /// То же без `spawn_blocking`, для сервисов, которые и так выполняются в блокирующем потоке.
    ///
    /// # Errors
    /// `Locked` без сессии или ошибка из `f`.
    pub fn with_session_sync<T>(
        &self,
        f: impl FnOnce(&mut Db) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        self.current().ok_or(AppError::Locked)?.run(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [3; 32];

    fn state_in(dir: &tempfile::TempDir) -> AppState {
        AppState::new(AppPaths::new(dir.path().to_path_buf()))
    }

    fn open_db(state: &AppState) -> Db {
        Db::create(&state.paths().db(), &KEY).unwrap()
    }

    fn block_on<T>(fut: impl std::future::Future<Output = T>) -> T {
        tauri::async_runtime::block_on(fut)
    }

    fn one(db: &mut Db) -> Result<i64, AppError> {
        db.conn()
            .query_row("SELECT 1", [], |r| r.get(0))
            .map_err(|e| AppError::internal("test", &e))
    }

    #[test]
    fn new_state_is_locked_and_does_not_run_the_closure() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let res = block_on(state.with_session(move |_| {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }));
        assert!(matches!(res, Err(AppError::Locked)));
        assert!(!ran.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!state.is_unlocked());
    }

    #[test]
    fn security_settings_reach_the_timer_the_window_flag_and_the_login_mirror() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert!(!state.lock_on_minimize());
        crate::services::settings::apply_security(&state, 15, true);
        assert!(state.lock_on_minimize());
        assert_eq!(
            crate::prefs::load(&state.paths().prefs()).autolock_minutes,
            Some(15)
        );
        // 0 минут — «никогда»: таймер не истекает.
        crate::services::settings::apply_security(&state, 0, false);
        assert!(!state.lock_on_minimize());
        assert!(!state.idle().expired(crate::idle::Moment::now()));
    }

    #[test]
    fn open_session_runs_closure_against_the_db() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.open_session(open_db(&state));
        assert!(state.is_unlocked());
        assert_eq!(block_on(state.with_session(one)).unwrap(), 1);
    }

    #[test]
    fn lock_closes_the_session_and_commands_get_locked() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.open_session(open_db(&state));
        assert!(state.lock());
        assert!(!state.lock(), "second lock is a no-op");
        assert!(matches!(
            block_on(state.with_session(one)),
            Err(AppError::Locked)
        ));
    }

    #[test]
    fn closure_error_is_returned_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.open_session(open_db(&state));
        let res: Result<(), AppError> = block_on(state.with_session(|_| {
            Err(AppError::Conflict {
                message_key: "errors.test".into(),
            })
        }));
        assert!(matches!(res, Err(AppError::Conflict { .. })));
    }

    #[test]
    fn panic_in_closure_becomes_internal_not_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.open_session(open_db(&state));
        let res: Result<(), AppError> = block_on(state.with_session(|_| panic!("boom")));
        assert!(matches!(res, Err(AppError::Internal { .. })));
        // Мьютекс отравлен паникой, но сессия остаётся рабочей.
        assert_eq!(block_on(state.with_session(one)).unwrap(), 1);
    }

    #[test]
    fn locking_drops_the_db_so_the_file_can_be_reopened_with_another_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.open_session(open_db(&state));
        state.lock();
        state.open_session(open_db(&state));
        assert_eq!(block_on(state.with_session(one)).unwrap(), 1);
    }
}

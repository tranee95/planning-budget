use chrono::{Duration, TimeZone};
use tempfile::TempDir;

use super::*;
use crate::state::AppPaths;

const FAST: KdfParams = KdfParams {
    m_kib: 8,
    t: 1,
    p: 1,
};
const PASSWORD: &str = "correct horse battery";

fn pw(s: &str) -> SecretString {
    SecretString::from(s)
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap()
}

/// Каталог объявлен первым, чтобы удаляться после состояния (на Windows
/// открытая БД держит файл).
fn fresh() -> (TempDir, AppState) {
    let dir = TempDir::new().unwrap();
    let state = AppState::new(AppPaths::new(dir.path().to_path_buf()));
    (dir, state)
}

fn created() -> (TempDir, AppState, Zeroizing<String>) {
    let (dir, state) = fresh();
    let code = create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    (dir, state, code)
}

fn categories(state: &AppState) -> i64 {
    tauri::async_runtime::block_on(state.with_session(|db| {
        db.conn()
            .query_row("SELECT count(*) FROM categories", [], |r| r.get(0))
            .map_err(|e| AppError::internal("test", &e))
    }))
    .unwrap()
}

#[test]
fn status_before_and_after_create() {
    let (_dir, state) = fresh();
    let before = status(&state, t0()).unwrap();
    assert!(!before.exists);
    assert!(before.locked);

    create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    let after = status(&state, t0()).unwrap();
    assert!(after.exists);
    assert!(!after.locked);
    assert_eq!(after.retry_after_ms, 0);
}

#[test]
fn create_opens_a_seeded_session_and_allows_saving_the_code_once() {
    let (_dir, state, code) = created();
    assert_eq!(categories(&state), 17);
    assert!(state.recovery_save_allowed());
    assert_eq!(code.split('-').count(), 7);
}

#[test]
fn create_twice_is_a_conflict() {
    let (_dir, state, _code) = created();
    let err = create(&state, &pw("another long password"), t0(), Some(FAST)).unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }));
}

#[test]
fn create_rejects_a_short_password_without_leaving_files() {
    let (dir, state) = fresh();
    let err = create(&state, &pw("short"), t0(), Some(FAST)).unwrap_err();
    assert!(matches!(err, AppError::Validation { .. }));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn create_refuses_to_adopt_an_orphan_database() {
    let (dir, state) = fresh();
    fs::write(dir.path().join("budget.db"), b"orphan").unwrap();
    let err = create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }));
    assert!(
        dir.path().join("budget.db").exists(),
        "orphan is not deleted"
    );
}

#[test]
fn lock_then_unlock_restores_access_to_the_same_data() {
    let (_dir, state, _code) = created();
    assert!(state.lock());
    let locked = tauri::async_runtime::block_on(state.with_session(|_| Ok(())));
    assert!(matches!(locked, Err(AppError::Locked)));

    unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert_eq!(categories(&state), 17);
    assert!(
        !state.recovery_save_allowed(),
        "lock revokes saving the code"
    );
}

#[test]
fn wrong_password_reports_backoff_after_three_failures() {
    let (_dir, state, _code) = created();
    state.lock();
    for attempt in 1..=3 {
        let err = unlock(&state, &pw("wrong password!"), t0()).unwrap_err();
        let AppError::WrongPassword { retry_after_ms } = err else {
            panic!("expected WrongPassword, got {err:?}");
        };
        assert_eq!(retry_after_ms, if attempt < 3 { 0 } else { 1000 });
    }
    let blocked = unlock(&state, &pw(PASSWORD), t0()).unwrap_err();
    assert!(matches!(
        blocked,
        AppError::TooManyAttempts {
            retry_after_ms: 1000
        }
    ));
    assert_eq!(status(&state, t0()).unwrap().retry_after_ms, 1000);

    unlock(&state, &pw(PASSWORD), t0() + Duration::seconds(2)).unwrap();
}

#[test]
fn unlock_with_an_open_session_still_checks_the_password() {
    let (_dir, state, _code) = created();
    let err = unlock(&state, &pw("wrong password!"), t0()).unwrap_err();
    assert!(matches!(err, AppError::WrongPassword { .. }));
    assert!(state.is_unlocked(), "a failed check keeps the session");
    unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert!(state.is_unlocked());
}

#[test]
fn unlock_applies_the_autolock_setting() {
    let (_dir, state, _code) = created();
    tauri::async_runtime::block_on(state.with_session(|db| {
        db.conn()
            .execute(
                "UPDATE settings SET value = '0' WHERE key = 'security.autolock_minutes'",
                [],
            )
            .map_err(|e| AppError::internal("test", &e))
    }))
    .unwrap();
    state.lock();
    unlock(&state, &pw(PASSWORD), t0()).unwrap();
    let far = crate::idle::Moment {
        monotonic: std::time::Instant::now() + std::time::Duration::from_secs(7 * 24 * 3600),
        wall: std::time::SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600),
    };
    assert!(!state.idle().expired(far), "0 minutes = never");
}

#[test]
fn change_password_swaps_the_secret_but_keeps_the_data() {
    let (_dir, state, _code) = created();
    state.lock();
    let new = pw("a completely new secret");
    let wrong = change_password(&state, &pw("wrong old password"), &new, t0()).unwrap_err();
    assert!(matches!(wrong, AppError::WrongPassword { .. }));

    change_password(&state, &pw(PASSWORD), &new, t0()).unwrap();
    assert!(matches!(
        unlock(&state, &pw(PASSWORD), t0()).unwrap_err(),
        AppError::WrongPassword { .. }
    ));
    unlock(&state, &new, t0()).unwrap();
    assert_eq!(categories(&state), 17);
}

#[test]
fn recovery_code_unlocks_and_sets_a_new_password() {
    let (_dir, state, code) = created();
    state.lock();
    let new = pw("forgotten and replaced");
    unlock_recovery(&state, &pw(&code), &new, t0()).unwrap();
    assert_eq!(categories(&state), 17);

    state.lock();
    unlock(&state, &new, t0()).unwrap();
}

#[test]
fn bad_recovery_code_is_a_validation_error() {
    let (_dir, state, _code) = created();
    state.lock();
    let new = pw("forgotten and replaced");
    for bad in ["nonsense", "AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AA"] {
        let err = unlock_recovery(&state, &pw(bad), &new, t0()).unwrap_err();
        assert!(matches!(err, AppError::Validation { .. }), "{bad}");
    }
    assert!(!state.is_unlocked());
}

#[test]
fn reset_needs_phrase_and_password_then_removes_everything() {
    let (dir, state, _code) = created();
    let phrase_err = reset(&state, &pw(PASSWORD), "нет", t0()).unwrap_err();
    assert!(matches!(phrase_err, AppError::Validation { .. }));
    let pw_err = reset(&state, &pw("wrong password!"), RESET_PHRASE, t0()).unwrap_err();
    assert!(matches!(pw_err, AppError::WrongPassword { .. }));
    assert!(state.is_unlocked(), "nothing happened yet");

    reset(&state, &pw(PASSWORD), "  Удалить ВСЕ данные ", t0()).unwrap();
    assert!(!state.is_unlocked());
    // Остаётся только файл оболочки (тема, масштаб): данных и ключей в нём нет.
    let left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| name != "ui-prefs.json")
        .collect();
    assert!(left.is_empty(), "{left:?}");
    assert!(!status(&state, t0()).unwrap().exists);
}

#[test]
fn recovery_file_is_written_once_and_only_when_allowed() {
    let (dir, state, code) = created();
    let target = dir.path().join("code.txt");
    let target2 = target.clone();

    assert!(
        !save_recovery_code(&state, &pw(&code), || None).unwrap(),
        "dialog cancelled"
    );
    assert!(state.recovery_save_allowed(), "cancel keeps the right");

    assert!(save_recovery_code(&state, &pw(&code), move || Some(target2)).unwrap());
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        format!("{}\n", code.as_str())
    );

    let again = save_recovery_code(&state, &pw(&code), || Some(target.clone())).unwrap_err();
    assert!(matches!(again, AppError::Conflict { .. }));
}

#[cfg(unix)]
#[test]
fn recovery_file_is_private_to_the_owner() {
    use std::os::unix::fs::PermissionsExt as _;

    let (dir, state, code) = created();
    // Файл, перезаписанный через диалог, уже существует с чужими правами.
    let target = dir.path().join("code.txt");
    fs::write(&target, b"old").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
    let t = target.clone();
    assert!(save_recovery_code(&state, &pw(&code), move || Some(t)).unwrap());
    let mode = fs::metadata(&target).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);

    let (dir2, state2, code2) = created();
    let fresh_file = dir2.path().join("new.txt");
    let t = fresh_file.clone();
    assert!(save_recovery_code(&state2, &pw(&code2), move || Some(t)).unwrap());
    let mode = fs::metadata(&fresh_file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn save_is_refused_when_the_session_never_issued_a_code() {
    let (_dir, state) = fresh();
    let err = save_recovery_code(&state, &pw("x"), || None).unwrap_err();
    assert!(matches!(err, AppError::Conflict { .. }));
}

/// Прежняя папка с сейфом рядом с новой; `AppState` смотрит на новую.
fn with_legacy() -> (TempDir, AppState) {
    let root = TempDir::new().unwrap();
    let old = root.path().join("old");
    let new = root.path().join("new");
    fs::create_dir_all(&old).unwrap();
    fs::create_dir_all(&new).unwrap();
    {
        let legacy = AppState::new(AppPaths::new(old.clone()));
        create(&legacy, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
        legacy.lock();
    }
    let state = AppState::new(AppPaths::new(new));
    state.set_legacy_dir(Some(old));
    (root, state)
}

#[test]
fn failed_migration_is_reported_and_retry_completes_it() {
    let (_root, state) = with_legacy();
    let blocker = state.paths().data_dir().join("vault.json.migrating");
    fs::create_dir_all(&blocker).unwrap();

    run_migration(&state);
    assert!(state.migration_failed());
    assert!(status(&state, t0()).unwrap().migration_failed);

    let err = retry_migration(&state, t0()).unwrap_err();
    assert!(
        matches!(err, AppError::Io { ref message_key } if message_key == "errors.io.migration")
    );
    assert!(state.migration_failed());

    fs::remove_dir(&blocker).unwrap();
    let after = retry_migration(&state, t0()).unwrap();
    assert!(!after.migration_failed);
    assert!(after.exists);
    assert!(state.paths().data_dir().join("vault.json").is_file());
}

#[test]
fn creating_a_new_vault_clears_the_failed_migration_flag() {
    let (_dir, state) = fresh();
    state.set_migration_failed(true);
    create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    assert!(!state.migration_failed());
}

#[test]
fn migration_without_legacy_dir_does_nothing() {
    let (_dir, state) = fresh();
    run_migration(&state);
    assert!(!state.migration_failed());
    assert!(!status(&state, t0()).unwrap().migration_failed);
}

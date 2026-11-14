use chrono::{Duration, TimeZone};
use planning_budget_vault::KdfParams;
use tempfile::TempDir;

use super::*;
use crate::services::{vault, vaults};
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
    Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0).unwrap()
}

fn open_vault() -> (TempDir, AppState) {
    let dir = TempDir::new().unwrap();
    let state = AppState::new(AppPaths::new(dir.path().to_path_buf()));
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    state
        .with_session_sync(|db| Ok(db.seed_categories(t0(), None)?))
        .unwrap();
    (dir, state)
}

fn category_count(state: &AppState) -> i64 {
    state
        .with_session_sync(|db| {
            db.conn()
                .query_row("SELECT count(*) FROM categories", [], |r| r.get(0))
                .map_err(|e| AppError::internal("test", &e))
        })
        .unwrap()
}

#[test]
fn a_backup_opens_as_a_new_vault_and_leaves_the_original_alone() {
    let (_dir, state) = open_vault();
    let original = state.paths().active_dir();
    let made = create(&state, t0()).unwrap();
    assert!(made.file_name.starts_with("main-20261101-120000"));

    import_named(&state, &made.file_name, &pw(PASSWORD), "Из копии", t0()).unwrap();
    assert!(!state.is_unlocked(), "import locks the session");
    assert_ne!(state.paths().active_dir(), original);

    // Хранилище из копии входит тем же паролем и содержит те же данные.
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert!(category_count(&state) > 0);
    // Оригинал на месте и открывается.
    vaults::switch(&state, MAIN_ID).unwrap();
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
    assert!(category_count(&state) > 0);
    assert_eq!(vaults::list(&state).len(), 2);
}

#[test]
fn wrong_password_imports_nothing_and_leaves_no_temp_folder() {
    let (dir, state) = open_vault();
    let made = create(&state, t0()).unwrap();
    let err = import_named(&state, &made.file_name, &pw("not the password"), "X", t0());
    assert!(matches!(err, Err(AppError::WrongPassword { .. })));
    assert_eq!(vaults::list(&state).len(), 1);
    assert!(state.is_unlocked(), "a failed import keeps the session");
    let left: Vec<_> = fs::read_dir(dir.path().join("vaults"))
        .map(|d| d.flatten().collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn the_backup_file_holds_no_plain_data() {
    let (_dir, state) = open_vault();
    state
        .with_session_sync(|db| {
            db.conn()
                .execute(
                    "UPDATE categories SET name = 'УникальнаяСтрокаПроверки' WHERE id = (SELECT min(id) FROM categories)",
                    [],
                )
                .map_err(|e| AppError::internal("test", &e))
        })
        .unwrap();
    let made = create(&state, t0()).unwrap();
    let (dir, _) = dir(&state);
    let bytes = fs::read(dir.join(made.file_name)).unwrap();
    let has = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
    assert!(!has("УникальнаяСтрокаПроверки".as_bytes()));
    assert!(!has(b"SQLite format 3"));
}

#[test]
fn rotation_keeps_the_last_seven_of_this_vault_only() {
    let (_dir, state) = open_vault();
    for day in 0..9 {
        create(&state, t0() + Duration::days(day)).unwrap();
    }
    let shown = status(&state);
    assert_eq!(shown.items.len(), KEEP);
    assert!(shown.items[0].file_name.contains("20261109"));
    assert!(shown.dir_is_default);

    // Копии другого хранилища в той же папке ротация не трогает.
    vaults::add(&state, "Второе").unwrap();
    vault::create(&state, &pw(PASSWORD), t0(), Some(FAST)).unwrap();
    create(&state, t0()).unwrap();
    assert_eq!(status(&state).items.len(), 1);
    vaults::switch(&state, MAIN_ID).unwrap();
    assert_eq!(status(&state).items.len(), KEEP);
}

#[test]
fn schedule_decides_when_an_automatic_copy_is_due() {
    let day = |d| NaiveDate::from_ymd_opt(2026, 11, d).unwrap();
    assert!(is_due(None, day(10), 7));
    assert!(!is_due(Some(day(10)), day(10), 1), "same day");
    assert!(is_due(Some(day(10)), day(11), 1));
    assert!(!is_due(Some(day(10)), day(16), 7));
    assert!(is_due(Some(day(10)), day(17), 7));
}

#[test]
fn auto_backup_follows_the_enable_flag_and_the_interval() {
    let (_dir, state) = open_vault();
    state
        .with_session_sync(|db| {
            crate::services::settings::set(
                db,
                crate::dto::SettingsPatchDto {
                    backup_auto: Some(false),
                    ..Default::default()
                },
            )
            .map(|_| ())
        })
        .unwrap();
    run_auto(&state, t0());
    assert!(status(&state).items.is_empty(), "disabled");

    state
        .with_session_sync(|db| {
            crate::services::settings::set(
                db,
                crate::dto::SettingsPatchDto {
                    backup_auto: Some(true),
                    backup_interval_days: Some(7),
                    ..Default::default()
                },
            )
            .map(|_| ())
        })
        .unwrap();
    run_auto(&state, t0());
    run_auto(&state, t0() + Duration::days(3));
    assert_eq!(status(&state).items.len(), 1, "inside the interval");
    run_auto(&state, t0() + Duration::days(7));
    assert_eq!(status(&state).items.len(), 2);
}

#[test]
fn names_from_the_interface_cannot_point_outside_the_backup_folder() {
    let (_dir, state) = open_vault();
    for bad in [
        "../x.budgetbak",
        "a\\b.budgetbak",
        "C:\\x.budgetbak",
        // Буква диска без корня: на Windows `join` заменил бы каталог копий.
        "C:x.budgetbak",
        "D:planted.budgetbak",
        // Альтернативный поток данных NTFS.
        "x:y.budgetbak",
        "x.zip",
    ] {
        let err = import_named(&state, bad, &pw(PASSWORD), "X", t0());
        assert!(matches!(err, Err(AppError::Validation { .. })), "{bad}");
    }
}

#[test]
fn a_custom_directory_is_used_and_must_be_writable() {
    let (_dir, state) = open_vault();
    let other = TempDir::new().unwrap();
    set_dir(&state, other.path().to_path_buf()).unwrap();
    let made = create(&state, t0()).unwrap();
    assert!(other.path().join(&made.file_name).is_file());
    assert!(!status(&state).dir_is_default);
    assert!(set_dir(&state, other.path().join("missing")).is_err());
    reset_dir(&state).unwrap();
    assert!(status(&state).dir_is_default);
}

#[test]
fn a_copy_made_during_a_backoff_still_opens_with_the_right_password() {
    let (_dir, state) = open_vault();
    for _ in 0..4 {
        let _ = vault::unlock(&state, &pw("wrong password!"), t0());
    }
    let made = create(&state, t0()).unwrap();
    import_named(&state, &made.file_name, &pw(PASSWORD), "Из копии", t0()).unwrap();
    vault::unlock(&state, &pw(PASSWORD), t0()).unwrap();
}

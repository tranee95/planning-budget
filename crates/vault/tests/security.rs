//! Часть чек-листа безопасности, относящаяся к сейфу.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "вспомогательные функции интеграционных тестов лежат вне #[test]"
)]

use std::fs;
use std::path::Path;

use budget_vault::{KdfParams, VaultError, VaultStore, backoff_secs};
use chrono::{DateTime, Duration, TimeZone, Utc};
use secrecy::SecretString;
use tempfile::TempDir;

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

fn new_vault() -> (TempDir, VaultStore, budget_vault::Created) {
    let dir = TempDir::new().unwrap();
    let store = VaultStore::new(dir.path().to_path_buf());
    let created = store.create_with_params(&pw(PASSWORD), FAST, t0()).unwrap();
    (dir, store, created)
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn create_then_unlock_returns_same_dek() {
    let (_dir, store, created) = new_vault();
    let dek = store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());
}

#[test]
fn create_leaves_only_vault_json() {
    let (dir, _store, _created) = new_vault();
    assert_eq!(file_names(dir.path()), ["vault.json"]);
}

#[test]
fn vault_json_has_no_plaintext_secrets() {
    let (dir, _store, created) = new_vault();
    let text = fs::read_to_string(dir.path().join("vault.json")).unwrap();
    let dek_b64 = data_encoding::BASE64.encode(created.dek.as_bytes());
    let dek_hex = data_encoding::HEXLOWER.encode(created.dek.as_bytes());
    assert!(!text.contains(&dek_b64));
    assert!(!text.contains(&dek_hex));
    assert!(!text.contains(PASSWORD));
    assert!(!text.contains(created.recovery_code.display().as_str()));
}

#[test]
fn second_create_is_refused() {
    let (_dir, store, _created) = new_vault();
    let err = store
        .create_with_params(&pw("another long password"), FAST, t0())
        .unwrap_err();
    assert!(matches!(err, VaultError::AlreadyExists));
}

#[test]
fn short_password_is_refused() {
    let dir = TempDir::new().unwrap();
    let store = VaultStore::new(dir.path().to_path_buf());
    let err = store
        .create_with_params(&pw("123456789"), FAST, t0())
        .unwrap_err();
    assert!(matches!(err, VaultError::PasswordTooShort { min: 10 }));
    assert!(!store.status(t0()).unwrap().exists);
}

#[test]
fn wrong_password_is_rejected() {
    let (_dir, store, _created) = new_vault();
    let err = store.unlock(&pw("not the password"), t0()).unwrap_err();
    assert!(matches!(err, VaultError::WrongPassword));
}

#[test]
fn backoff_schedule_matches_doc() {
    let expected = [
        (0, 0),
        (2, 0),
        (3, 1),
        (4, 2),
        (5, 4),
        (8, 32),
        (9, 60),
        (50, 60),
    ];
    for (attempts, secs) in expected {
        assert_eq!(backoff_secs(attempts), secs, "attempts = {attempts}");
    }
}

#[test]
fn backoff_starts_after_third_failure_and_survives_restart() {
    let (dir, store, _created) = new_vault();
    for _ in 0..3 {
        let err = store.unlock(&pw("wrong password!"), t0()).unwrap_err();
        assert!(matches!(err, VaultError::WrongPassword));
    }

    // «Перезапуск»: новый объект над тем же каталогом.
    let restarted = VaultStore::new(dir.path().to_path_buf());
    let status = restarted.status(t0()).unwrap();
    assert_eq!(status.retry_after_secs, 1);

    // Даже верный пароль в окне задержки не принимается.
    let err = restarted.unlock(&pw(PASSWORD), t0()).unwrap_err();
    assert!(matches!(
        err,
        VaultError::Backoff {
            retry_after_secs: 1
        }
    ));

    let later = t0() + Duration::seconds(2);
    restarted.unlock(&pw(PASSWORD), later).unwrap();
    assert_eq!(restarted.status(later).unwrap().retry_after_secs, 0);
}

#[test]
fn backoff_is_capped_at_sixty_seconds() {
    let (_dir, store, _created) = new_vault();
    let mut last_attempt = t0();
    for _ in 0..12 {
        let _ = store.unlock(&pw("wrong password!"), last_attempt);
        last_attempt += Duration::seconds(61);
    }
    last_attempt -= Duration::seconds(61);
    assert_eq!(store.status(last_attempt).unwrap().retry_after_secs, 60);
    let after = last_attempt + Duration::seconds(61);
    assert_eq!(store.status(after).unwrap().retry_after_secs, 0);
}

#[test]
fn successful_unlock_resets_counter() {
    let (_dir, store, _created) = new_vault();
    for _ in 0..2 {
        let _ = store.unlock(&pw("wrong password!"), t0());
    }
    store.unlock(&pw(PASSWORD), t0()).unwrap();
    for _ in 0..2 {
        let _ = store.unlock(&pw("wrong password!"), t0());
    }
    assert_eq!(store.status(t0()).unwrap().retry_after_secs, 0);
}

#[test]
fn change_password_keeps_dek_and_drops_old_password() {
    let (dir, store, created) = new_vault();
    let new = "a completely new secret";
    store
        .change_password(&pw(PASSWORD), &pw(new), t0())
        .unwrap();

    let dek = store.unlock(&pw(new), t0()).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());
    let err = store.unlock(&pw(PASSWORD), t0()).unwrap_err();
    assert!(matches!(err, VaultError::WrongPassword));
    assert_eq!(
        file_names(dir.path()),
        ["vault.json"],
        "no .bak or .tmp left"
    );
}

#[test]
fn change_password_requires_correct_old_password() {
    let (_dir, store, _created) = new_vault();
    let err = store
        .change_password(
            &pw("wrong old password"),
            &pw("a completely new secret"),
            t0(),
        )
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongPassword));
    store.unlock(&pw(PASSWORD), t0()).unwrap();
}

#[test]
fn recovery_code_unlocks_and_sets_new_password() {
    let (_dir, store, created) = new_vault();
    let code = SecretString::from(created.recovery_code.display().to_string());

    let dek = store.unlock_recovery(&code).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());

    let new = "forgotten and replaced";
    let dek = store.reset_password_with_recovery(&code, &pw(new)).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());
    store.unlock(&pw(new), t0()).unwrap();
    assert!(matches!(
        store.unlock(&pw(PASSWORD), t0()).unwrap_err(),
        VaultError::WrongPassword
    ));
}

#[test]
fn recovery_reset_clears_backoff() {
    let (_dir, store, created) = new_vault();
    for _ in 0..4 {
        let _ = store.unlock(&pw("wrong password!"), t0());
    }
    assert!(store.status(t0()).unwrap().retry_after_secs > 0);
    let code = SecretString::from(created.recovery_code.display().to_string());
    store
        .reset_password_with_recovery(&code, &pw("brand new password"))
        .unwrap();
    assert_eq!(store.status(t0()).unwrap().retry_after_secs, 0);
}

#[test]
fn wrong_or_malformed_recovery_code_is_rejected() {
    let (_dir, store, _created) = new_vault();
    let other = budget_vault_other_code();
    assert!(matches!(
        store.unlock_recovery(&pw(&other)).unwrap_err(),
        VaultError::WrongRecoveryCode
    ));
    assert!(matches!(
        store.unlock_recovery(&pw("not a code")).unwrap_err(),
        VaultError::RecoveryCodeFormat
    ));
}

/// Валидный по формату код от другого сейфа.
fn budget_vault_other_code() -> String {
    let dir = TempDir::new().unwrap();
    let other = VaultStore::new(dir.path().to_path_buf());
    let created = other
        .create_with_params(&pw("some other password"), FAST, t0())
        .unwrap();
    created.recovery_code.display().to_string()
}

#[test]
fn leftover_bak_is_shredded_when_main_file_is_intact() {
    let (dir, store, _created) = new_vault();
    fs::copy(
        dir.path().join("vault.json"),
        dir.path().join("vault.json.bak"),
    )
    .unwrap();
    fs::write(dir.path().join("vault.json.tmp"), b"partial").unwrap();

    store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(file_names(dir.path()), ["vault.json"]);
}

#[test]
fn bak_is_restored_when_main_file_is_missing() {
    let (dir, store, _created) = new_vault();
    fs::rename(
        dir.path().join("vault.json"),
        dir.path().join("vault.json.bak"),
    )
    .unwrap();

    store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(file_names(dir.path()), ["vault.json"]);
}

#[test]
fn tampered_wrapped_key_gives_wrong_password_not_a_panic() {
    let (dir, store, _created) = new_vault();
    let path = dir.path().join("vault.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    json["pw"]["wrapped_dek"] = serde_json::Value::String(data_encoding::BASE64.encode(&[7u8; 48]));
    fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

    assert!(matches!(
        store.unlock(&pw(PASSWORD), t0()).unwrap_err(),
        VaultError::WrongPassword
    ));
}

#[test]
fn vault_id_is_bound_to_wrapped_key() {
    let (dir, store, _created) = new_vault();
    let path = dir.path().join("vault.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    json["vault_id"] = serde_json::Value::String("00000000-0000-4000-8000-000000000000".into());
    fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

    assert!(matches!(
        store.unlock(&pw(PASSWORD), t0()).unwrap_err(),
        VaultError::WrongPassword
    ));
}

#[test]
fn absurd_kdf_params_are_rejected_before_hashing() {
    let (dir, store, _created) = new_vault();
    let path = dir.path().join("vault.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    json["kdf"]["m_kib"] = serde_json::Value::from(u32::MAX);
    fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

    assert!(matches!(
        store.unlock(&pw(PASSWORD), t0()).unwrap_err(),
        VaultError::Corrupt
    ));
}

#[test]
fn destroy_removes_the_vault_and_allows_a_fresh_one() {
    let (dir, store, _created) = new_vault();
    store.destroy().unwrap();
    assert!(file_names(dir.path()).is_empty());
    assert!(!store.status(t0()).unwrap().exists);
    store.destroy().unwrap();
    store
        .create_with_params(&pw("another long password"), FAST, t0())
        .unwrap();
}

#[test]
fn missing_vault_reports_not_found() {
    let dir = TempDir::new().unwrap();
    let store = VaultStore::new(dir.path().to_path_buf());
    assert!(!store.status(t0()).unwrap().exists);
    assert!(matches!(
        store.unlock(&pw(PASSWORD), t0()).unwrap_err(),
        VaultError::NotFound
    ));
}

#[test]
fn debug_output_hides_secrets() {
    let (_dir, _store, created) = new_vault();
    let text = format!("{created:?}");
    assert!(text.contains("redacted"));
    assert!(!text.contains(created.recovery_code.display().as_str()));
}

#[test]
fn rekey_swaps_dek_and_recovery_code_and_drops_the_old_ones() {
    let (dir, store, created) = new_vault();
    let old_json = fs::read(dir.path().join("vault.json")).unwrap();
    let old_code = SecretString::from(created.recovery_code.display().to_string());

    let plan = store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    assert_ne!(plan.new_dek.as_bytes(), created.dek.as_bytes());
    assert!(store.rekey_pending().unwrap());
    // До шага 3 старый вход ещё работает: база могла остаться под старым ключом.
    let dek = store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());

    store.finish_rekey().unwrap();
    assert!(!store.rekey_pending().unwrap());
    assert_eq!(file_names(dir.path()), ["vault.json"]);

    let dek = store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(dek.as_bytes(), plan.new_dek.as_bytes());
    assert!(matches!(
        store.unlock_recovery(&old_code).unwrap_err(),
        VaultError::WrongRecoveryCode
    ));
    let new_code = SecretString::from(plan.recovery_code.display().to_string());
    let dek = store.unlock_recovery(&new_code).unwrap();
    assert_eq!(dek.as_bytes(), plan.new_dek.as_bytes());

    // Старый vault.json со старым паролем по-прежнему даёт СТАРЫЙ DEK:
    // он больше не подходит к базе, и это проверяет тест в src-tauri.
    fs::write(dir.path().join("vault.json"), old_json).unwrap();
    let dek = store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());
}

#[test]
fn rekey_requires_the_current_password() {
    let (_dir, store, _created) = new_vault();
    let err = store.begin_rekey(&pw("wrong password!"), t0()).unwrap_err();
    assert!(matches!(err, VaultError::WrongPassword));
    assert!(!store.rekey_pending().unwrap());
}

#[test]
fn abort_rekey_restores_the_plain_file() {
    let (_dir, store, created) = new_vault();
    store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    store.abort_rekey().unwrap();
    assert!(!store.rekey_pending().unwrap());
    let dek = store.unlock(&pw(PASSWORD), t0()).unwrap();
    assert_eq!(dek.as_bytes(), created.dek.as_bytes());
}

#[test]
fn pending_rekey_dek_is_available_with_the_password_only() {
    let (_dir, store, _created) = new_vault();
    assert!(store.pending_rekey_dek(&pw(PASSWORD)).unwrap().is_none());
    let plan = store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    let dek = store.pending_rekey_dek(&pw(PASSWORD)).unwrap().unwrap();
    assert_eq!(dek.as_bytes(), plan.new_dek.as_bytes());
    assert!(matches!(
        store.pending_rekey_dek(&pw("wrong password!")).unwrap_err(),
        VaultError::WrongPassword
    ));
}

#[test]
fn status_never_touches_files_on_disk() {
    let (dir, store, _created) = new_vault();
    fs::copy(
        dir.path().join("vault.json"),
        dir.path().join("vault.json.bak"),
    )
    .unwrap();
    fs::write(dir.path().join("vault.json.tmp"), b"partial").unwrap();
    let before = file_names(dir.path());

    assert!(store.status(t0()).unwrap().exists);
    assert_eq!(file_names(dir.path()), before, "status is read-only");
}

#[test]
fn status_reads_the_backup_when_the_main_file_is_gone() {
    let (dir, store, _created) = new_vault();
    fs::rename(
        dir.path().join("vault.json"),
        dir.path().join("vault.json.bak"),
    )
    .unwrap();
    assert!(store.status(t0()).unwrap().exists);
    assert_eq!(file_names(dir.path()), ["vault.json.bak"]);
}

#[test]
fn clock_moved_back_does_not_extend_the_lockout() {
    let (_dir, store, _created) = new_vault();
    let wrong_clock = t0() + Duration::days(365);
    for _ in 0..3 {
        let _ = store.unlock(&pw("wrong password!"), wrong_clock);
    }
    // Часы исправили: ждать нужно не год, а не дольше обычной задержки.
    let err = store.unlock(&pw(PASSWORD), t0()).unwrap_err();
    assert!(
        matches!(
            err,
            VaultError::Backoff {
                retry_after_secs: 1
            }
        ),
        "{err:?}"
    );
    store
        .unlock(&pw(PASSWORD), t0() + Duration::seconds(2))
        .unwrap();
}

#[test]
fn second_rekey_is_refused_while_the_first_is_pending() {
    let (_dir, store, _created) = new_vault();
    let plan = store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    let err = store.begin_rekey(&pw(PASSWORD), t0()).unwrap_err();
    assert!(matches!(err, VaultError::RekeyPending));
    // Слоты первого перевыпуска целы.
    let dek = store.pending_rekey_dek(&pw(PASSWORD)).unwrap().unwrap();
    assert_eq!(dek.as_bytes(), plan.new_dek.as_bytes());
}

#[test]
fn change_password_during_pending_rekey_keeps_the_new_dek_reachable() {
    let (_dir, store, created) = new_vault();
    let plan = store.begin_rekey(&pw(PASSWORD), t0()).unwrap();
    let new = "a completely new secret";
    store
        .change_password(&pw(PASSWORD), &pw(new), t0())
        .unwrap();

    // Оба ключа открываются новым паролем и не открываются старым.
    let old_dek = store.unlock(&pw(new), t0()).unwrap();
    assert_eq!(old_dek.as_bytes(), created.dek.as_bytes());
    let next_dek = store.pending_rekey_dek(&pw(new)).unwrap().unwrap();
    assert_eq!(next_dek.as_bytes(), plan.new_dek.as_bytes());
    assert!(matches!(
        store.pending_rekey_dek(&pw(PASSWORD)).unwrap_err(),
        VaultError::WrongPassword
    ));

    store.finish_rekey().unwrap();
    let dek = store.unlock(&pw(new), t0()).unwrap();
    assert_eq!(dek.as_bytes(), plan.new_dek.as_bytes());
}

//! Релизный профиль не должен возвращать `panic = "abort"`: `with_session` ловит панику
//! команды, и с `abort` этот перехват в поставляемой сборке не работает.

#![allow(
    clippy::unwrap_used,
    reason = "вспомогательные функции интеграционных тестов лежат вне #[test]"
)]

use std::fs;
use std::path::Path;

#[test]
fn release_profile_unwinds_so_session_panic_recovery_works() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let text = fs::read_to_string(manifest).unwrap();
    let release = text
        .split("[profile.release]")
        .nth(1)
        .expect("в Cargo.toml нет [profile.release]");
    let section = release.split("\n[").next().unwrap();
    let compact: String = section.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !compact.contains("panic=\"abort\""),
        "panic = \"abort\" в релизе отключает перехват паники в with_session"
    );
}

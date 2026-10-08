fn main() {
    tauri_build::build();
    embed_test_manifest();
}

/// Тестовые exe, которые линкуют Tauri, на Windows не стартуют без манифеста Common Controls v6
/// (`STATUS_ENTRYPOINT_NOT_FOUND`): `tauri-build` встраивает его только в основной бинарник.
fn embed_test_manifest() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|e| e == "msvc")
        && std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if !msvc {
        return;
    }
    let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("windows-test.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest.display()
    );
}

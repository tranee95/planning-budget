//! Команды вызываются через настоящую диспетчеризацию Tauri (mock-рантайм), а не только как функции:
//! `tests/locked.rs` проверяет исходники, а здесь — что запрос по IPC без сессии получает `Locked`.
//! Команды с явным `AppHandle` привязаны к `Wry` и сюда не попадают; берутся команды с одним `State`.

#![allow(clippy::unwrap_used, reason = "тест")]

use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager as _, WebviewWindow, WebviewWindowBuilder};

use planning_budget_app_lib::commands;
use planning_budget_app_lib::{AppPaths, AppState};

/// Адрес локального окна: на Windows и Android это `http://tauri.localhost`, на остальных — `tauri://localhost`.
/// Иначе Tauri считает запрос внешним и отвечает «Plugin not found».
#[cfg(any(windows, target_os = "android"))]
const LOCAL_ORIGIN: &str = "http://tauri.localhost";
#[cfg(not(any(windows, target_os = "android")))]
const LOCAL_ORIGIN: &str = "tauri://localhost";

fn app_with_state(dir: &tempfile::TempDir) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            commands::categories::categories_list,
            commands::debts::debts_list,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    app.manage(AppState::new(AppPaths::new(dir.path().to_path_buf())));
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    (app, window)
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: LOCAL_ORIGIN.parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn data_commands_answer_locked_over_ipc_without_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let (_app, window) = app_with_state(&dir);

    let err = invoke(
        &window,
        "categories_list",
        json!({ "includeArchived": false }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "Locked", "{err}");
}

#[test]
fn the_same_commands_answer_with_data_while_open_and_locked_again_after_lock() {
    let dir = tempfile::tempdir().unwrap();
    let (app, window) = app_with_state(&dir);
    let state = app.state::<AppState>();
    let key = [7u8; 32];
    let db = planning_budget_storage::Db::create(&state.paths().db(), &key).unwrap();
    state.open_session(db);

    let list = invoke(
        &window,
        "categories_list",
        json!({ "includeArchived": false }),
    )
    .unwrap();
    assert!(list.is_array(), "{list}");

    assert!(state.lock());
    for (cmd, body) in [
        ("categories_list", json!({ "includeArchived": false })),
        (
            "debts_list",
            json!({ "month": "2026-10", "includeClosed": false }),
        ),
    ] {
        let err = invoke(&window, cmd, body).unwrap_err();
        assert_eq!(err["code"], "Locked", "{cmd}: {err}");
    }
}
